use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;

use anyhow::Context;
use codex_mcp_server::CodexToolCallParam;

use pretty_assertions::assert_eq;
use rmcp::model::CallToolRequestParam;
use rmcp::model::ClientCapabilities;
use rmcp::model::ElicitationCapability;
use rmcp::model::Implementation;
use rmcp::model::InitializeRequestParam;
use rmcp::model::JsonRpcRequest;
use rmcp::model::JsonRpcResponse;
use rmcp::model::JsonRpcVersion2_0;
use rmcp::model::ProtocolVersion;
use rmcp::model::RequestId;
use serde_json::json;
use tokio::process::Command;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct LooseJsonRpcNotification {
    pub jsonrpc: JsonRpcVersion2_0,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

pub struct McpProcess {
    next_request_id: AtomicI64,
    /// Retain this child process until the client is dropped. The Tokio runtime
    /// will make a "best effort" to reap the process after it exits, but it is
    /// not a guarantee. See the `kill_on_drop` documentation for details.
    #[allow(dead_code)]
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    pub async fn new(codex_home: &Path) -> anyhow::Result<Self> {
        Self::new_with_env(codex_home, &[]).await
    }

    /// Creates a new MCP process, allowing tests to override or remove
    /// specific environment variables for the child process only.
    ///
    /// Pass a tuple of (key, Some(value)) to set/override, or (key, None) to
    /// remove a variable from the child's environment.
    pub async fn new_with_env(
        codex_home: &Path,
        env_overrides: &[(&str, Option<&str>)],
    ) -> anyhow::Result<Self> {
        let program = codex_utils_cargo_bin::cargo_bin("codex-mcp-server")
            .context("should find binary for codex-mcp-server")?;
        let mut cmd = Command::new(program);

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env("CODEX_HOME", codex_home);
        cmd.env("RUST_LOG", "debug");

        for (k, v) in env_overrides {
            match v {
                Some(val) => {
                    cmd.env(k, val);
                }
                None => {
                    cmd.env_remove(k);
                }
            }
        }

        let mut process = cmd
            .kill_on_drop(true)
            .spawn()
            .context("codex-mcp-server proc should start")?;
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::format_err!("mcp should have stdin fd"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow::format_err!("mcp should have stdout fd"))?;
        let stdout = BufReader::new(stdout);

        // Forward child's stderr to our stderr so failures are visible even
        // when stdout/stderr are captured by the test harness.
        if let Some(stderr) = process.stderr.take() {
            let mut stderr_reader = BufReader::new(stderr).lines();
            tokio::spawn(async move {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    eprintln!("[mcp stderr] {line}");
                }
            });
        }
        Ok(Self {
            next_request_id: AtomicI64::new(0),
            process,
            stdin,
            stdout,
        })
    }

    /// Performs the initialization handshake with the MCP server.
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        let params = InitializeRequestParam {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ClientCapabilities {
                // We only need to signal support; exact shape isn't important for these tests.
                elicitation: Some(ElicitationCapability::default()),
                experimental: None,
                roots: None,
                sampling: None,
            },
            client_info: Implementation {
                name: "elicitation test".into(),
                title: Some("Elicitation Test".into()),
                version: "0.0.0".into(),
                icons: None,
                website_url: None,
            },
        };
        let params_value = serde_json::to_value(params)?;

        self.send_jsonrpc_message(json!({
            "jsonrpc": JsonRpcVersion2_0,
            "id": RequestId::Number(request_id),
            "method": "initialize",
            "params": params_value,
        }))
        .await?;

        let initialized = self.read_jsonrpc_message().await?;
        let os_info = os_info::get();
        let build_version = env!("CARGO_PKG_VERSION");
        let originator = codex_core::default_client::originator().value;
        let user_agent = format!(
            "{originator}/{build_version} ({} {}; {}) {} (elicitation test; 0.0.0)",
            os_info.os_type(),
            os_info.version(),
            os_info.architecture().unwrap_or("unknown"),
            codex_core::terminal::user_agent()
        );
        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "capabilities": {
                        "tools": {
                            "listChanged": true
                        },
                    },
                    "serverInfo": {
                        "name": "codex-mcp-server",
                        "title": "Codex",
                        "version": build_version,
                        "user_agent": user_agent
                    },
                    "protocolVersion": ProtocolVersion::LATEST
                }
            }),
            initialized
        );

        // Send notifications/initialized to ack the response.
        self.send_jsonrpc_message(json!({
            "jsonrpc": JsonRpcVersion2_0,
            "method": "notifications/initialized",
        }))
        .await?;

        Ok(())
    }

    /// Returns the id used to make the request so it can be used when
    /// correlating notifications.
    pub async fn send_codex_tool_call(
        &mut self,
        params: CodexToolCallParam,
    ) -> anyhow::Result<i64> {
        let arguments = serde_json::to_value(params)?;
        let arguments = match arguments {
            serde_json::Value::Object(map) => Some(map),
            other => anyhow::bail!("codex tool call arguments must be an object, got {other:?}"),
        };

        let codex_tool_call_params = CallToolRequestParam {
            name: "codex".into(),
            arguments,
        };

        self.send_request(
            "tools/call",
            Some(serde_json::to_value(codex_tool_call_params)?),
        )
        .await
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<i64> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        self.send_jsonrpc_message(json!({
            "jsonrpc": JsonRpcVersion2_0,
            "id": RequestId::Number(request_id),
            "method": method,
            "params": params,
        }))
        .await?;
        Ok(request_id)
    }

    pub async fn send_response(
        &mut self,
        id: RequestId,
        result: serde_json::Value,
    ) -> anyhow::Result<()> {
        self.send_jsonrpc_message(json!({
            "jsonrpc": JsonRpcVersion2_0,
            "id": id,
            "result": result,
        }))
        .await
    }

    async fn send_jsonrpc_message(&mut self, message: serde_json::Value) -> anyhow::Result<()> {
        eprintln!("writing message to stdin: {message:?}");
        let payload = serde_json::to_string(&message)?;
        self.stdin.write_all(payload.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_jsonrpc_message(&mut self) -> anyhow::Result<serde_json::Value> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        let message = serde_json::from_str::<serde_json::Value>(&line)?;
        eprintln!("read message from stdout: {message:?}");
        Ok(message)
    }

    pub async fn read_stream_until_request_message(
        &mut self,
    ) -> anyhow::Result<JsonRpcRequest<rmcp::model::RequestOptionalParam<String, serde_json::Value>>>
    {
        eprintln!("in read_stream_until_request_message()");

        loop {
            let message = self.read_jsonrpc_message().await?;

            let obj = message
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("expected JSON-RPC object, got {message:?}"))?;

            if obj.contains_key("method") && obj.contains_key("id") {
                return Ok(serde_json::from_value(message)?);
            }
            if obj.contains_key("method") && !obj.contains_key("id") {
                eprintln!("notification: {message:?}");
                continue;
            }
            if obj.contains_key("result") {
                anyhow::bail!("unexpected JSON-RPC response: {message:?}");
            }
            if obj.contains_key("error") {
                anyhow::bail!("unexpected JSON-RPC error: {message:?}");
            }

            anyhow::bail!("unrecognized JSON-RPC message: {message:?}");
        }
    }

    pub async fn read_stream_until_response_message(
        &mut self,
        request_id: RequestId,
    ) -> anyhow::Result<JsonRpcResponse<serde_json::Value>> {
        eprintln!("in read_stream_until_response_message({request_id:?})");

        loop {
            let message = self.read_jsonrpc_message().await?;
            let obj = message
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("expected JSON-RPC object, got {message:?}"))?;

            if obj.contains_key("method") && !obj.contains_key("id") {
                eprintln!("notification: {message:?}");
                continue;
            }
            if obj.contains_key("method") && obj.contains_key("id") {
                anyhow::bail!("unexpected JSON-RPC request: {message:?}");
            }
            if obj.contains_key("error") {
                anyhow::bail!("unexpected JSON-RPC error: {message:?}");
            }
            if obj.contains_key("result") {
                let response: JsonRpcResponse<serde_json::Value> = serde_json::from_value(message)?;
                if response.id == request_id {
                    return Ok(response);
                }
            } else {
                anyhow::bail!("unrecognized JSON-RPC message: {message:?}");
            }
        }
    }

    /// Reads notifications until a legacy TurnComplete event is observed:
    /// Method "codex/event" with params.msg.type == "task_complete".
    pub async fn read_stream_until_legacy_task_complete_notification(
        &mut self,
    ) -> anyhow::Result<LooseJsonRpcNotification> {
        eprintln!("in read_stream_until_legacy_task_complete_notification()");

        loop {
            let message = self.read_jsonrpc_message().await?;
            let obj = message
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("expected JSON-RPC object, got {message:?}"))?;

            if obj.contains_key("method") && obj.contains_key("id") {
                anyhow::bail!("unexpected JSON-RPC request: {message:?}");
            }
            if obj.contains_key("result") {
                anyhow::bail!("unexpected JSON-RPC response: {message:?}");
            }
            if obj.contains_key("error") {
                anyhow::bail!("unexpected JSON-RPC error: {message:?}");
            }
            if !obj.contains_key("method") {
                anyhow::bail!("unrecognized JSON-RPC message: {message:?}");
            }

            let notification: LooseJsonRpcNotification = serde_json::from_value(message)?;

            let is_match = if notification.method == "codex/event" {
                if let Some(params) = &notification.params {
                    params
                        .get("msg")
                        .and_then(|m| m.get("type"))
                        .and_then(|t| t.as_str())
                        == Some("task_complete")
                } else {
                    false
                }
            } else {
                false
            };

            if is_match {
                return Ok(notification);
            }

            eprintln!("ignoring notification: {notification:?}");
        }
    }
}
