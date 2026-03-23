/*
 * Copyright 2026 ZeroClaw Community
 *
 * Licensed under the MIT License. See LICENSE in the project root.
 */

//! MCP tools for listing, inspecting, and calling MCP server tools.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use zeroclaw::tools::{Tool, ToolResult};

use crate::mcp_client::McpClient;

/// Lists all available tools from an MCP server.
///
/// Returns tool names, descriptions, and parameter schemas.
/// Use `mcp_get_tool_params` for detailed parameter info on a specific tool.
pub struct McpListToolsTool {
    client: Arc<McpClient>,
}

impl McpListToolsTool {
    pub fn new(client: Arc<McpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for McpListToolsTool {
    fn name(&self) -> &'static str {
        "mcp_list_tools"
    }

    fn description(&self) -> &'static str {
        "List all available tools from the connected MCP server. \
         Returns each tool's name and description. \
         Use mcp_get_tool_params to get detailed parameter info for a specific tool."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "refresh": {
                    "type": "boolean",
                    "description": "Force refresh the cached tool list (default: false)",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let force_refresh = args.get("refresh").and_then(|v| v.as_bool()).unwrap_or(false);

        match self.client.list_tools(force_refresh).await {
            Ok(tools) => {
                let summary: Vec<serde_json::Value> = tools
                    .into_iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description.unwrap_or_default(),
                        })
                    })
                    .collect();

                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&summary)
                        .unwrap_or_else(|_| "[]".to_string()),
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to list MCP tools: {e}")),
            }),
        }
    }
}

/// Gets the parameter schema for a specific MCP tool.
///
/// Returns the tool's description and JSON input schema, useful for
/// understanding how to call the tool with `mcp_call_tool`.
pub struct McpGetToolParamsTool {
    client: Arc<McpClient>,
}

impl McpGetToolParamsTool {
    pub fn new(client: Arc<McpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for McpGetToolParamsTool {
    fn name(&self) -> &'static str {
        "mcp_get_tool_params"
    }

    fn description(&self) -> &'static str {
        "Get detailed parameter schema for a specific MCP tool by name. \
         Returns the tool's description and JSON input schema showing \
         required and optional parameters with their types."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "The name of the MCP tool to inspect"
                }
            },
            "required": ["tool_name"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let tool_name = args
            .get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'tool_name' parameter"))?;

        match self.client.get_tool_params(tool_name).await {
            Ok(info) => {
                let result = json!({
                    "name": info.name,
                    "description": info.description.unwrap_or_default(),
                    "input_schema": info.input_schema,
                });

                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "{}".to_string()),
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to get MCP tool params: {e}")),
            }),
        }
    }
}

/// Calls a tool on the MCP server with the given arguments.
///
/// Executes the named tool with the provided JSON arguments and returns
/// the tool's result content.
pub struct McpCallToolTool {
    client: Arc<McpClient>,
}

impl McpCallToolTool {
    pub fn new(client: Arc<McpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for McpCallToolTool {
    fn name(&self) -> &'static str {
        "mcp_call_tool"
    }

    fn description(&self) -> &'static str {
        "Call a tool on the MCP server. Provide the tool name and arguments \
         as a JSON object. Use mcp_list_tools to discover available tools \
         and mcp_get_tool_params to see the required parameter schema."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "The name of the MCP tool to call"
                },
                "arguments": {
                    "type": "object",
                    "description": "Arguments to pass to the tool, as a JSON object matching the tool's input schema",
                    "additionalProperties": true
                }
            },
            "required": ["tool_name", "arguments"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let tool_name = args
            .get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'tool_name' parameter"))?;

        let arguments = args
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        match self.client.call_tool(tool_name, arguments).await {
            Ok(output) => Ok(ToolResult {
                success: true,
                output,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to call MCP tool: {e}")),
            }),
        }
    }
}
