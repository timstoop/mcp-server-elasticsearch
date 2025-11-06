// Licensed to Elasticsearch B.V. under one or more contributor
// license agreements. See the NOTICE file distributed with
// this work for additional information regarding copyright
// ownership. Elasticsearch B.V. licenses this file to you under
// the Apache License, Version 2.0 (the "License"); you may
// not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::net::{TcpListener, SocketAddr};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct PortForwardConfig {
    pub namespace: String,
    pub service: String,
    pub local_port: u16,
    pub remote_port: u16,
}

impl Default for PortForwardConfig {
    fn default() -> Self {
        Self {
            namespace: "infra".to_string(),
            service: "logs-es-http".to_string(),
            local_port: 9200,
            remote_port: 9200,
        }
    }
}

impl PortForwardConfig {
    pub fn from_env() -> Self {
        let desired_port = std::env::var("K8S_LOCAL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9200);

        let local_port = find_available_port(desired_port);

        if local_port != desired_port {
            tracing::info!(
                "Port {} not available, using port {} instead",
                desired_port,
                local_port
            );
        }

        Self {
            namespace: std::env::var("K8S_NAMESPACE").unwrap_or_else(|_| "infra".to_string()),
            service: std::env::var("K8S_SERVICE").unwrap_or_else(|_| "logs-es-http".to_string()),
            local_port,
            remote_port: std::env::var("K8S_REMOTE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9200),
        }
    }

    pub fn es_url(&self) -> String {
        format!("http://localhost:{}", self.local_port)
    }
}

fn is_port_available(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).is_ok()
}

fn find_available_port(preferred_port: u16) -> u16 {
    // First try the preferred port
    if is_port_available(preferred_port) {
        return preferred_port;
    }

    // Try nearby ports (preferred +/- 10)
    for offset in 1..=10 {
        // Try preferred + offset
        let port_up = preferred_port.saturating_add(offset);
        if port_up != preferred_port && is_port_available(port_up) {
            return port_up;
        }

        // Try preferred - offset
        if let Some(port_down) = preferred_port.checked_sub(offset) {
            if is_port_available(port_down) {
                return port_down;
            }
        }
    }

    // If nothing found in range, find any available port
    // Try common high ports
    for port in 19200..19300 {
        if is_port_available(port) {
            return port;
        }
    }

    // Last resort: let OS assign a port
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return addr.port();
        }
    }

    // Fallback to preferred port (will likely fail, but at least we tried)
    preferred_port
}

pub async fn start_port_forward(config: PortForwardConfig) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<()>(1);

    tokio::spawn(async move {
        let mut retry_delay = Duration::from_secs(1);
        let max_retry_delay = Duration::from_secs(30);

        loop {
            tracing::info!(
                "Starting port-forward: kubectl port-forward -n {} svc/{} {}:{}",
                config.namespace,
                config.service,
                config.local_port,
                config.remote_port
            );

            match run_port_forward(&config).await {
                Ok(_) => {
                    tracing::warn!("Port-forward process exited normally");
                }
                Err(e) => {
                    tracing::error!("Port-forward error: {}", e);
                }
            }

            // Check if we should stop
            if rx.try_recv().is_ok() {
                tracing::info!("Port-forward shutdown requested");
                break;
            }

            tracing::info!("Restarting port-forward in {:?}", retry_delay);
            sleep(retry_delay).await;

            // Exponential backoff
            retry_delay = std::cmp::min(retry_delay * 2, max_retry_delay);
        }
    });

    // Drop the sender so the task can detect shutdown
    drop(tx);

    Ok(())
}

async fn run_port_forward(config: &PortForwardConfig) -> anyhow::Result<()> {
    let mut child = Command::new("kubectl")
        .arg("port-forward")
        .arg("-n")
        .arg(&config.namespace)
        .arg(format!("svc/{}", config.service))
        .arg(format!("{}:{}", config.local_port, config.remote_port))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    // Spawn tasks to read stdout and stderr
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::info!("kubectl stdout: {}", line);
        }
    });

    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            // Check for common error patterns
            if line.contains("error") || line.contains("Error") {
                tracing::error!("kubectl stderr: {}", line);
            } else {
                tracing::info!("kubectl stderr: {}", line);
            }
        }
    });

    // Wait for the process to complete
    let status = child.wait().await?;

    // Clean up tasks
    stdout_task.abort();
    stderr_task.abort();

    if !status.success() {
        anyhow::bail!("kubectl port-forward exited with status: {}", status);
    }

    Ok(())
}

pub fn should_enable_port_forward() -> bool {
    std::env::var("K8S_PORT_FORWARD")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false)
}
