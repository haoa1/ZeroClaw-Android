/*
 * Copyright 2026 ZeroClaw Community
 *
 * Licensed under the MIT License. See LICENSE in the project root.
 */

//! MCP (Model Context Protocol) client using SSE (Server-Sent Events) transport.
//!
//! MCP SSE protocol:
//! 1. GET /sse → opens SSE stream, server sends `endpoint` event with POST URL
//! 2. POST to endpoint URL with JSON-RPC requests
//! 3. Server sends responses back as SSE events

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, RwLock};

/// MCP tool definition returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// MCP client connected to a server via SSE.
pub struct McpClient {
    sse_url: String,
    client: reqwest::Client,
    tools_cache: Arc<RwLock<Option<Vec<McpToolInfo>>>>,
    session_id: Arc<Mutex<Option<String>>>,
    post_endpoint: Arc<Mutex<Option<String>>>,
}

/// JSON-RPC request structure.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

/// JSON-RPC response structure.
#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error object.
#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl McpClient {
    /// Creates a new MCP client pointing at the given SSE URL.
    pub fn new(sse_url: &str) -> Self {
        Self {
            sse_url: sse_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("failed to build MCP HTTP client"),
            tools_cache: Arc::new(RwLock::new(None)),
            session_id: Arc::new(Mutex::new(None)),
            post_endpoint: Arc::new(Mutex::new(None)),
        }
    }

    /// Connects to the SSE endpoint and performs MCP initialization.
    ///
    /// MCP SSE initialization flow:
    /// 1. GET /sse → server sends `endpoint` event with POST URL
    /// 2. POST initialize → server responds with capabilities
    /// 3. POST notifications/initialized → session ready
    async fn ensure_connected(&self) -> Result<String, String> {
        // Check if we already have an endpoint
        {
            let endpoint = self.post_endpoint.lock().await;
            if let Some(ref ep) = *endpoint {
                return Ok(ep.clone());
            }
        }

        tracing::info!("Connecting to MCP SSE endpoint: {}", self.sse_url);

        // Connect to SSE endpoint
        let response = self
            .client
            .get(&self.sse_url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| format!("MCP SSE connection failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("MCP SSE returned status: {}", response.status()));
        }

        let mut stream = response.bytes_stream();
        let mut endpoint_url: Option<String> = None;
        let mut session_id: Option<String> = None;

        // Read SSE events until we get the endpoint
        let mut buffer = String::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("SSE stream error: {e}"))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            for line in buffer.split('\n') {
                let line = line.trim();

                if line.starts_with("data: /message") || line.starts_with("data: http") {
                    let data = line.trim_start_matches("data: ");
                    endpoint_url = Some(data.to_string());
                    // Extract session_id from URL
                    if let Some(sid_start) = data.find("session_id=") {
                        let sid = &data[sid_start + 11..];
                        session_id = sid.split(|c: char| c == '&' || c == ' ').next().map(String::from);
                    }
                    tracing::info!("Got MCP endpoint: {data}");
                }
            }

            if endpoint_url.is_some() {
                break;
            }

            if buffer.len() > 4096 {
                let last_newline = buffer.rfind('\n').unwrap_or(0);
                buffer = buffer[last_newline..].to_string();
            }
        }

        let endpoint = endpoint_url.ok_or_else(|| {
            "MCP SSE: did not receive endpoint event. Server may not be an MCP server.".to_string()
        })?;

        // Build full endpoint URL
        let full_endpoint = if endpoint.starts_with("http") {
            endpoint
        } else {
            let base = self.sse_url.trim_end_matches("/sse");
            if endpoint.starts_with('/') {
                format!("{base}{endpoint}")
            } else {
                format!("{base}/{endpoint}")
            }
        };

        // Store session ID
        if let Some(sid) = session_id {
            let mut sid_guard = self.session_id.lock().await;
            *sid_guard = Some(sid);
        }

        {
            let mut ep_guard = self.post_endpoint.lock().await;
            *ep_guard = Some(full_endpoint.clone());
        }

        // Perform MCP initialization
        let init_result = self
            .call_rpc_inner(
                &full_endpoint,
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "zeroclaw",
                        "version": "0.1.0"
                    }
                }),
            )
            .await;

        match init_result {
            Ok(_) => {
                tracing::info!("MCP initialize succeeded");
                // Send initialized notification (no response expected)
                let _ = self
                    .send_notification(&full_endpoint, "notifications/initialized")
                    .await;
            }
            Err(e) => {
                tracing::warn!("MCP initialize failed: {e}");
                // Continue anyway - some servers don't require initialization
            }
        }

        Ok(full_endpoint)
    }

    /// Sends a JSON-RPC notification (no id, no response expected).
    async fn send_notification(&self, endpoint: &str, method: &str) -> Result<(), String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": {}
        });

        let mut url = endpoint.to_string();
        {
            let sid = self.session_id.lock().await;
            if let Some(ref session_id) = *sid {
                if !url.contains("session_id=") {
                    let separator = if url.contains('?') { '&' } else { '?' };
                    url.push_str(&format!("{separator}sessionId={session_id}"));
                }
            }
        }

        let _ = self.client.post(&url).json(&request).send().await;
        Ok(())
    }

    /// Internal RPC call with explicit endpoint.
    async fn call_rpc_inner(&self, endpoint: &str, method: &str, params: Value) -> Result<Value, String> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: method.to_string(),
            params,
        };

        let mut url = endpoint.to_string();
        {
            let sid = self.session_id.lock().await;
            if let Some(ref session_id) = *sid {
                if !url.contains("session_id=") {
                    let separator = if url.contains('?') { '&' } else { '?' };
                    url.push_str(&format!("{separator}sessionId={session_id}"));
                }
            }
        }

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("MCP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("MCP server returned {status}: {body}"));
        }

        // Response may be "Accepted" for async processing via SSE
        let body_text = response.text().await.unwrap_or_default();
        if body_text == "Accepted" {
            return Ok(serde_json::json!({"accepted": true}));
        }

        let rpc: JsonRpcResponse = serde_json::from_str(&body_text)
            .map_err(|e| format!("MCP response parse failed: {e} body: {body_text}"))?;

        if let Some(err) = rpc.error {
            return Err(format!("MCP RPC error ({}): {}", err.code, err.message));
        }

        rpc.result
            .ok_or_else(|| "MCP response missing result".to_string())
    }

    /// Sends a JSON-RPC request and returns the raw result value.
    async fn call_rpc(&self, method: &str, params: Value) -> Result<Value, String> {
        let endpoint = self.ensure_connected().await?;
        self.call_rpc_inner(&endpoint, method, params).await
    }

    /// Lists all available tools from the MCP server.
    pub async fn list_tools(&self, force_refresh: bool) -> Result<Vec<McpToolInfo>, String> {
        if !force_refresh {
            let cache = self.tools_cache.read().await;
            if let Some(ref tools) = *cache {
                return Ok(tools.clone());
            }
        }

        let result = self.call_rpc("tools/list", serde_json::json!({})).await?;

        let tools: Vec<McpToolInfo> = result
            .get("tools")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut cache = self.tools_cache.write().await;
        *cache = Some(tools.clone());

        Ok(tools)
    }

    /// Gets the parameter schema for a specific tool by name.
    pub async fn get_tool_params(&self, tool_name: &str) -> Result<McpToolInfo, String> {
        let tools = self.list_tools(false).await?;

        tools
            .into_iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| format!("MCP tool '{tool_name}' not found"))
    }

    /// Calls a tool on the MCP server with the given arguments.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<String, String> {
        let result = self
            .call_rpc(
                "tools/call",
                serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments
                }),
            )
            .await?;

        // MCP returns content as an array of content blocks
        if let Some(content) = result.get("content") {
            if let Some(arr) = content.as_array() {
                let mut output = String::new();
                for item in arr {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            if !output.is_empty() {
                                output.push('\n');
                            }
                            output.push_str(text);
                        }
                    }
                }
                if !output.is_empty() {
                    return Ok(output);
                }
            }
            serde_json::to_string(content).map_err(|e| format!("serialize content: {e}"))
        } else {
            serde_json::to_string(&result).map_err(|e| format!("serialize result: {e}"))
        }
    }
}

/// Default MCP SSE server URL.
const DEFAULT_MCP_URL: &str = "http://localhost:8888/sse";

/// Global MCP client singleton.
static MCP_CLIENT: OnceLock<Arc<McpClient>> = OnceLock::new();

/// Returns the global MCP client instance, creating it on first call.
///
/// Connects to the default MCP server at `http://localhost:8888/sse`.
/// Can be overridden by setting the `MCP_SERVER_URL` environment variable.
pub fn get_mcp_client() -> Arc<McpClient> {
    MCP_CLIENT
        .get_or_init(|| {
            let url = std::env::var("MCP_SERVER_URL")
                .unwrap_or_else(|_| DEFAULT_MCP_URL.to_string());
            tracing::info!("Initializing MCP client with SSE URL: {url}");
            Arc::new(McpClient::new(&url))
        })
        .clone()
}
