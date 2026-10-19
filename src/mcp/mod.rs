use clap::Subcommand;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

pub mod tools;

#[derive(Subcommand, Debug)]
pub enum McpCommand {
    /// Run MCP stdio server for LLM integration
    Serve,
}

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn run(action: McpCommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        McpCommand::Serve => serve_stdio().await,
    }
}

async fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "tick-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "notifications/initialized" => {}
            "ping" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/list" => {
                let tools_list = tools::list_tools();
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools_list
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/call" => {
                let tool_name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let empty_args = json!({});
                let arguments = req.params.get("arguments").unwrap_or(&empty_args);

                match tools::call_tool(tool_name, arguments).await {
                    Ok(val) => {
                        let text_content = if let Some(s) = val.as_str() {
                            s.to_string()
                        } else {
                            serde_json::to_string_pretty(&val).unwrap_or_default()
                        };
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text_content
                                    }
                                ]
                            }
                        });
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                    Err(e) => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Error: {e}")
                                    }
                                ]
                            }
                        });
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                }
            }
            _ => {
                if req.id.is_some() {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("Method not found: {}", req.method) }
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                    stdout.flush()?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tools_contains_expected_jira_and_confluence_tools() {
        let tools_val = tools::list_tools();
        let arr = tools_val.as_array().expect("tools array");
        assert!(arr.len() >= 25);
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        let expected = [
            "jira_search",
            "jira_get_issue",
            "jira_create_issue",
            "jira_edit_issue",
            "jira_delete_issue",
            "jira_list_transitions",
            "jira_transition_issue",
            "jira_assign_issue",
            "jira_add_comment",
            "jira_list_comments",
            "jira_delete_comment",
            "jira_add_worklog",
            "jira_list_worklogs",
            "jira_delete_worklog",
            "jira_link_issues",
            "jira_list_projects",
            "jira_get_project",
            "jira_list_project_components",
            "jira_list_project_versions",
            "jira_list_boards",
            "jira_list_sprints",
            "jira_list_users",
            "jira_get_current_user",
            "confluence_list_spaces",
            "confluence_get_space",
            "confluence_list_pages",
            "confluence_search",
            "confluence_get_page",
            "confluence_create_page",
            "confluence_update_page",
            "confluence_delete_page",
            "confluence_list_comments",
            "confluence_add_comment",
            "confluence_list_page_versions",
            "confluence_list_attachments",
            "atlassian_raw_api",
        ];

        for tool_name in expected {
            assert!(names.contains(&tool_name), "Missing MCP tool: {tool_name}");
        }

        // Validate schema structure of each tool
        for tool in arr {
            let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
            assert!(!name.is_empty(), "Tool must have name");
            let desc = tool
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            assert!(!desc.is_empty(), "Tool {name} must have description");
            let schema = tool.get("inputSchema").expect("inputSchema");
            assert_eq!(schema.get("type").and_then(|t| t.as_str()), Some("object"));
            assert!(schema.get("properties").is_some());
        }
    }
}
