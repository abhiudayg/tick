use crate::api::confluence::storage_format_to_plain_text;
use crate::api::types::Ticket;
use crate::api::JiraClient;
use crate::cli::util::IssueJson;
use crate::config::{Config, Site};
use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!([
        {
            "name": "jira_search",
            "description": "Search Jira issues using JQL (Jira Query Language)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jql": { "type": "string", "description": "JQL query string (e.g. 'project = ENG AND status = Open')" },
                    "max_results": { "type": "integer", "description": "Maximum number of issues to return (default: 25)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["jql"]
            }
        },
        {
            "name": "jira_get_issue",
            "description": "Fetch detailed information for a specific Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_create_issue",
            "description": "Create a new Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Project key (e.g. ENG)" },
                    "issue_type": { "type": "string", "description": "Issue type name (e.g. Task, Bug, Story)" },
                    "summary": { "type": "string", "description": "Summary / title of the issue" },
                    "description": { "type": "string", "description": "Description in markdown" },
                    "priority": { "type": "string", "description": "Optional priority name" },
                    "assignee": { "type": "string", "description": "Optional assignee accountId or 'me'" },
                    "labels": { "type": "array", "items": { "type": "string" }, "description": "List of labels" },
                    "parent": { "type": "string", "description": "Parent issue key if subtask" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["project", "issue_type", "summary"]
            }
        },
        {
            "name": "jira_edit_issue",
            "description": "Edit fields of an existing Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "summary": { "type": "string", "description": "New summary" },
                    "description": { "type": "string", "description": "New description in markdown" },
                    "priority": { "type": "string", "description": "New priority name" },
                    "assignee": { "type": "string", "description": "Assignee accountId, 'me', or 'none'" },
                    "labels": { "type": "array", "items": { "type": "string" }, "description": "New list of labels" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_delete_issue",
            "description": "Delete a Jira issue permanently",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_list_transitions",
            "description": "List available workflow transitions for a Jira issue including required fields and allowed options",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_transition_issue",
            "description": "Transition Jira issue to a new workflow status with optional resolution, custom fields, and comment",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "status": { "type": "string", "description": "Target status name (e.g. 'In Progress', 'Done') or transition ID" },
                    "resolution": { "type": "string", "description": "Resolution name (e.g. 'Done', 'Fixed', 'Won\\'t Fix')" },
                    "fields": { "type": "object", "description": "Key-value map of transition screen fields (e.g. custom fields)" },
                    "comment": { "type": "string", "description": "Optional comment to include with the transition" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "status"]
            }
        },
        {
            "name": "jira_assign_issue",
            "description": "Assign a Jira issue to a user or clear assignee",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "assignee": { "type": "string", "description": "User account ID, 'me', or 'none'" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "assignee"]
            }
        },
        {
            "name": "jira_add_comment",
            "description": "Add a comment to a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "body": { "type": "string", "description": "Comment body in markdown" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "body"]
            }
        },
        {
            "name": "jira_list_comments",
            "description": "List comments for a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key (e.g. PROJ-123)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_delete_comment",
            "description": "Delete a comment from a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key" },
                    "comment_id": { "type": "string", "description": "Comment ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "comment_id"]
            }
        },
        {
            "name": "jira_add_worklog",
            "description": "Log work spent on a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key" },
                    "time_spent": { "type": "string", "description": "Time spent string (e.g. '2h 30m', '1d')" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "time_spent"]
            }
        },
        {
            "name": "jira_list_worklogs",
            "description": "List worklogs logged on a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_delete_worklog",
            "description": "Delete a worklog entry from a Jira issue",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Issue key" },
                    "worklog_id": { "type": "string", "description": "Worklog ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key", "worklog_id"]
            }
        },
        {
            "name": "jira_link_issues",
            "description": "Create an issue link between two Jira issues",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "inward_key": { "type": "string", "description": "Inward issue key" },
                    "outward_key": { "type": "string", "description": "Outward issue key" },
                    "link_type": { "type": "string", "description": "Link type (e.g. 'Relates', 'Blocks', 'Clones')", "default": "Relates" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["inward_key", "outward_key"]
            }
        },
        {
            "name": "jira_list_projects",
            "description": "List all accessible Jira projects",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "site": { "type": "string", "description": "Optional site name from config" }
                }
            }
        },
        {
            "name": "jira_get_project",
            "description": "Get Jira project details and metadata",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Project key (e.g. ENG)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_list_project_components",
            "description": "List components defined in a Jira project",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Project key (e.g. ENG)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_list_project_versions",
            "description": "List release versions defined in a Jira project",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Project key (e.g. ENG)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "jira_list_boards",
            "description": "List Jira agile boards",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "site": { "type": "string", "description": "Optional site name from config" }
                }
            }
        },
        {
            "name": "jira_list_sprints",
            "description": "List active and future sprints on an agile board",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "board_id": { "type": "integer", "description": "Agile board ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["board_id"]
            }
        },
        {
            "name": "jira_list_users",
            "description": "Search users in Jira by name or email",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Username, display name, or email search query" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "jira_get_current_user",
            "description": "Get current user profile and account ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "site": { "type": "string", "description": "Optional site name from config" }
                }
            }
        },
        {
            "name": "confluence_list_spaces",
            "description": "List Confluence spaces",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum number of spaces to return (default: 25)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                }
            }
        },
        {
            "name": "confluence_get_space",
            "description": "Get details of a specific Confluence space",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "space_id_or_key": { "type": "string", "description": "Space ID or key" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["space_id_or_key"]
            }
        },
        {
            "name": "confluence_list_pages",
            "description": "List pages in a Confluence space",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "space_id": { "type": "string", "description": "Numeric space ID or key" },
                    "limit": { "type": "integer", "description": "Maximum results (default: 25)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["space_id"]
            }
        },
        {
            "name": "confluence_search",
            "description": "Search Confluence pages using CQL or title keywords",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "CQL query or title keywords" },
                    "limit": { "type": "integer", "description": "Maximum results (default: 25)" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "confluence_get_page",
            "description": "Get Confluence page content and metadata",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id"]
            }
        },
        {
            "name": "confluence_create_page",
            "description": "Create a new page in a Confluence space",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "space_id": { "type": "string", "description": "Confluence space ID or key" },
                    "title": { "type": "string", "description": "Page title" },
                    "body": { "type": "string", "description": "Page body in markdown or plain text" },
                    "parent_id": { "type": "string", "description": "Optional parent page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["space_id", "title", "body"]
            }
        },
        {
            "name": "confluence_update_page",
            "description": "Update an existing Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "title": { "type": "string", "description": "Updated page title" },
                    "body": { "type": "string", "description": "Updated page content" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id", "title", "body"]
            }
        },
        {
            "name": "confluence_delete_page",
            "description": "Delete / archive a Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id"]
            }
        },
        {
            "name": "confluence_list_comments",
            "description": "List comments for a Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id"]
            }
        },
        {
            "name": "confluence_add_comment",
            "description": "Add a comment to a Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "body": { "type": "string", "description": "Comment content" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id", "body"]
            }
        },
        {
            "name": "confluence_list_page_versions",
            "description": "List version history of a Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id"]
            }
        },
        {
            "name": "confluence_list_attachments",
            "description": "List attachments uploaded to a Confluence page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "Confluence page ID" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["page_id"]
            }
        },
        {
            "name": "atlassian_raw_api",
            "description": "Execute an arbitrary REST API request against Jira or Confluence",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "endpoint": { "type": "string", "description": "REST endpoint (e.g. /rest/api/3/myself or /wiki/api/v2/spaces)" },
                    "method": { "type": "string", "description": "HTTP Method (GET, POST, PUT, DELETE)", "default": "GET" },
                    "body": { "type": "object", "description": "JSON request body for POST/PUT" },
                    "site": { "type": "string", "description": "Optional site name from config" }
                },
                "required": ["endpoint"]
            }
        }
    ])
}

async fn get_client_and_site(site_arg: Option<&str>) -> Result<(JiraClient, Site), String> {
    let config = Config::load().map_err(|e| format!("Config load error: {e}"))?;
    let site = if let Some(name) = site_arg {
        config
            .sites
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Site '{name}' not found"))?
    } else {
        config
            .sites
            .first()
            .cloned()
            .ok_or_else(|| "No sites configured".to_string())?
    };
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    Ok((client, site))
}

pub async fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    let site_arg = args.get("site").and_then(|v| v.as_str());
    let (client, site) = get_client_and_site(site_arg).await?;
    let base_url = site.base_url.clone();

    match name {
        "jira_search" => {
            let jql = args
                .get("jql")
                .and_then(|v| v.as_str())
                .ok_or("Missing jql")?;
            let max_results = args
                .get("max_results")
                .and_then(|v| v.as_u64())
                .unwrap_or(25) as u32;
            let mut progress = None;
            let ids = client
                .search_jql(&base_url, jql, max_results, &mut progress)
                .await?;
            let issues = client.bulk_fetch(&base_url, &ids, None, &[], false).await?;
            let tickets: Vec<IssueJson> = issues
                .into_iter()
                .map(|i| {
                    let ticket =
                        Ticket::from_bulk_fetch(i, &site.name, &base_url, None, &[], false);
                    IssueJson::from_ticket(&ticket, &site.name)
                })
                .collect();
            Ok(json!({ "issues": tickets, "count": tickets.len() }))
        }
        "jira_get_issue" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let issues = client
                .bulk_fetch(&base_url, &[key.to_string()], None, &[], true)
                .await?;
            let issue = issues
                .into_iter()
                .next()
                .ok_or_else(|| format!("Issue {key} not found"))?;
            let ticket = Ticket::from_bulk_fetch(issue, &site.name, &base_url, None, &[], true);
            let dto = IssueJson::from_ticket(&ticket, &site.name);
            Ok(serde_json::to_value(&dto).map_err(|e| e.to_string())?)
        }
        "jira_create_issue" => {
            let project = args
                .get("project")
                .and_then(|v| v.as_str())
                .ok_or("Missing project")?;
            let issue_type = args
                .get("issue_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing issue_type")?;
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .ok_or("Missing summary")?;
            let mut fields = json!({
                "project": { "key": project },
                "issuetype": { "name": issue_type },
                "summary": summary,
            });
            if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                fields["description"] = crate::api::markdown::to_adf(desc, &[]);
            }
            if let Some(prio) = args.get("priority").and_then(|v| v.as_str()) {
                if let Some(id) = client.resolve_priority_id(&base_url, prio).await {
                    fields["priority"] = json!({ "id": id });
                } else {
                    fields["priority"] = json!({ "name": prio });
                }
            }
            if let Some(assignee) = args.get("assignee").and_then(|v| v.as_str()) {
                let id = if assignee.eq_ignore_ascii_case("me") {
                    client.current_user_account_id(&base_url).await?
                } else {
                    assignee.to_string()
                };
                fields["assignee"] = json!({ "accountId": id });
            }
            if let Some(labels) = args.get("labels").and_then(|v| v.as_array()) {
                fields["labels"] = json!(labels);
            }
            if let Some(parent) = args.get("parent").and_then(|v| v.as_str()) {
                fields["parent"] = json!({ "key": parent });
            }
            let key = client
                .create_issue(&base_url, &fields)
                .await
                .map_err(|e| e.message)?;
            Ok(
                json!({ "key": key, "url": format!("{}/browse/{}", base_url.trim_end_matches('/'), key) }),
            )
        }
        "jira_edit_issue" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            if let Some(summary) = args.get("summary").and_then(|v| v.as_str()) {
                client.update_summary(&base_url, key, summary).await?;
            }
            if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                client.update_description(&base_url, key, desc, &[]).await?;
            }
            if let Some(prio) = args.get("priority").and_then(|v| v.as_str()) {
                client.update_priority(&base_url, key, prio).await?;
            }
            if let Some(assignee) = args.get("assignee").and_then(|v| v.as_str()) {
                if assignee.eq_ignore_ascii_case("none") {
                    client.unassign(&base_url, key).await?;
                } else if assignee.eq_ignore_ascii_case("me") {
                    let me_id = client.current_user_account_id(&base_url).await?;
                    client.assign_to_account(&base_url, key, &me_id).await?;
                } else {
                    client.assign_to_account(&base_url, key, assignee).await?;
                }
            }
            if let Some(labels_arr) = args.get("labels").and_then(|v| v.as_array()) {
                let labels: Vec<String> = labels_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                client.update_labels(&base_url, key, &labels).await?;
            }
            Ok(json!({ "key": key, "updated": true }))
        }
        "jira_delete_issue" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            client.delete_issue(&base_url, key).await?;
            Ok(json!({ "key": key, "deleted": true }))
        }
        "jira_list_transitions" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let transitions = crate::operations::transition::list_transitions_with_fields(
                &client, &base_url, key,
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "key": key, "transitions": transitions }))
        }
        "jira_transition_issue" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let status = args
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or("Missing status")?;
            let resolution = args.get("resolution").and_then(|v| v.as_str());
            let comment = args.get("comment").and_then(|v| v.as_str());
            let extra_fields: Option<std::collections::HashMap<String, serde_json::Value>> = args
                .get("fields")
                .and_then(|v| v.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

            let options = crate::operations::transition::TransitionOptions {
                resolution,
                extra_fields: extra_fields.as_ref(),
                comment,
            };
            crate::operations::transition::apply_transition_by_name_with_options(
                &client, &base_url, key, status, &options,
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "key": key, "status": status }))
        }
        "jira_assign_issue" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let assignee = args
                .get("assignee")
                .and_then(|v| v.as_str())
                .ok_or("Missing assignee")?;
            if assignee.eq_ignore_ascii_case("none") || assignee.eq_ignore_ascii_case("unassign") {
                client.unassign(&base_url, key).await?;
            } else if assignee.eq_ignore_ascii_case("me") {
                let me_id = client.current_user_account_id(&base_url).await?;
                client.assign_to_account(&base_url, key, &me_id).await?;
            } else {
                client.assign_to_account(&base_url, key, assignee).await?;
            }
            Ok(json!({ "key": key, "assignee": assignee }))
        }
        "jira_add_comment" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("Missing body")?;
            client
                .add_comment_with_attachments(&base_url, key, body, &[], &[])
                .await?;
            Ok(json!({ "key": key, "comment_added": true }))
        }
        "jira_list_comments" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let comments = client.list_comments(&base_url, key).await?;
            Ok(json!({ "key": key, "comments": comments }))
        }
        "jira_delete_comment" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let comment_id = args
                .get("comment_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing comment_id")?;
            client.delete_comment(&base_url, key, comment_id).await?;
            Ok(json!({ "key": key, "deleted_comment_id": comment_id }))
        }
        "jira_add_worklog" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let time_spent = args
                .get("time_spent")
                .and_then(|v| v.as_str())
                .ok_or("Missing time_spent")?;
            client.add_worklog(&base_url, key, time_spent).await?;
            Ok(json!({ "key": key, "time_spent": time_spent }))
        }
        "jira_list_worklogs" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let worklogs = client.list_worklogs(&base_url, key).await?;
            Ok(json!({ "key": key, "worklogs": worklogs }))
        }
        "jira_delete_worklog" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let worklog_id = args
                .get("worklog_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing worklog_id")?;
            client.delete_worklog(&base_url, key, worklog_id).await?;
            Ok(json!({ "key": key, "deleted_worklog_id": worklog_id }))
        }
        "jira_link_issues" => {
            let inward = args
                .get("inward_key")
                .and_then(|v| v.as_str())
                .ok_or("Missing inward_key")?;
            let outward = args
                .get("outward_key")
                .and_then(|v| v.as_str())
                .ok_or("Missing outward_key")?;
            let link_type = args
                .get("link_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Relates");
            client
                .link_issues(&base_url, link_type, inward, outward)
                .await?;
            Ok(json!({ "inward_key": inward, "outward_key": outward, "link_type": link_type }))
        }
        "jira_list_projects" => {
            let projects = client.list_projects(&base_url).await?;
            Ok(json!({ "projects": projects }))
        }
        "jira_get_project" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let project = client.get_project(&base_url, key).await?;
            Ok(project)
        }
        "jira_list_project_components" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let components = client.get_project_components(&base_url, key).await?;
            Ok(json!({ "key": key, "components": components }))
        }
        "jira_list_project_versions" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing key")?;
            let versions = client.get_project_versions(&base_url, key).await?;
            Ok(json!({ "key": key, "versions": versions }))
        }
        "jira_list_boards" => {
            let boards = client.list_boards(&base_url).await?;
            let serialized: Vec<Value> = boards
                .into_iter()
                .map(|b| json!({ "id": b.id, "name": b.name, "project_key": b.project_key }))
                .collect();
            Ok(json!({ "boards": serialized }))
        }
        "jira_list_sprints" => {
            let board_id = args
                .get("board_id")
                .and_then(|v| v.as_u64())
                .ok_or("Missing board_id")?;
            let sprints = client.list_board_sprints(&base_url, board_id).await?;
            let serialized: Vec<Value> = sprints
                .into_iter()
                .map(|(id, name, state)| json!({ "id": id, "name": name, "state": state }))
                .collect();
            Ok(json!({ "board_id": board_id, "sprints": serialized }))
        }
        "jira_list_users" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing query")?;
            let users = client.search_users(&base_url, query, Some(25)).await?;
            Ok(json!({ "users": users }))
        }
        "jira_get_current_user" => {
            let profile = client.get_user_profile(&base_url).await?;
            Ok(profile)
        }
        "confluence_list_spaces" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
            let spaces = client.confluence_list_spaces(&base_url, limit).await?;
            Ok(serde_json::to_value(&spaces).map_err(|e| e.to_string())?)
        }
        "confluence_get_space" => {
            let key = args
                .get("space_id_or_key")
                .and_then(|v| v.as_str())
                .ok_or("Missing space_id_or_key")?;
            let space = client.confluence_get_space(&base_url, key).await?;
            Ok(serde_json::to_value(&space).map_err(|e| e.to_string())?)
        }
        "confluence_list_pages" => {
            let space_id = args
                .get("space_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing space_id")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
            let pages = client
                .confluence_list_pages_in_space(&base_url, space_id, limit)
                .await?;
            Ok(serde_json::to_value(&pages).map_err(|e| e.to_string())?)
        }
        "confluence_search" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing query")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
            let pages = client
                .confluence_search_pages(&base_url, query, limit)
                .await?;
            Ok(serde_json::to_value(&pages).map_err(|e| e.to_string())?)
        }
        "confluence_get_page" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let page = client.confluence_get_page(&base_url, page_id).await?;
            let plain_body = page
                .body
                .as_ref()
                .and_then(|b| b.storage.as_ref())
                .map(|s| storage_format_to_plain_text(&s.value))
                .unwrap_or_default();
            let mut val = serde_json::to_value(&page).map_err(|e| e.to_string())?;
            if let Some(obj) = val.as_object_mut() {
                obj.insert("plain_text_body".to_string(), json!(plain_body));
            }
            Ok(val)
        }
        "confluence_create_page" => {
            let space_id = args
                .get("space_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing space_id")?;
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("Missing title")?;
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("Missing body")?;
            let parent_id = args.get("parent_id").and_then(|v| v.as_str());
            let storage = crate::api::confluence::plain_text_to_storage_format(body);
            let page = client
                .confluence_create_page(&base_url, space_id, title, &storage, parent_id)
                .await?;
            Ok(serde_json::to_value(&page).map_err(|e| e.to_string())?)
        }
        "confluence_update_page" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("Missing title")?;
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("Missing body")?;
            let existing = client.confluence_get_page(&base_url, page_id).await?;
            let next_ver = existing.version.as_ref().map(|v| v.number).unwrap_or(1) + 1;
            let storage = crate::api::confluence::plain_text_to_storage_format(body);
            let page = client
                .confluence_update_page(&base_url, page_id, title, &storage, next_ver)
                .await?;
            Ok(serde_json::to_value(&page).map_err(|e| e.to_string())?)
        }
        "confluence_delete_page" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            client.confluence_delete_page(&base_url, page_id).await?;
            Ok(json!({ "page_id": page_id, "deleted": true }))
        }
        "confluence_list_comments" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let comments = client.confluence_list_comments(&base_url, page_id).await?;
            Ok(serde_json::to_value(&comments).map_err(|e| e.to_string())?)
        }
        "confluence_add_comment" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("Missing body")?;
            let storage = crate::api::confluence::plain_text_to_storage_format(body);
            let comment = client
                .confluence_add_comment(&base_url, page_id, &storage)
                .await?;
            Ok(serde_json::to_value(&comment).map_err(|e| e.to_string())?)
        }
        "confluence_list_page_versions" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let versions = client
                .confluence_list_page_versions(&base_url, page_id)
                .await?;
            Ok(json!({ "page_id": page_id, "versions": versions }))
        }
        "confluence_list_attachments" => {
            let page_id = args
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing page_id")?;
            let attachments = client
                .confluence_list_attachments(&base_url, page_id)
                .await?;
            Ok(json!({ "page_id": page_id, "attachments": attachments }))
        }
        "atlassian_raw_api" => {
            let endpoint = args
                .get("endpoint")
                .and_then(|v| v.as_str())
                .ok_or("Missing endpoint")?;
            let method_str = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
            let method =
                reqwest::Method::from_bytes(method_str.as_bytes()).map_err(|e| e.to_string())?;
            let body = args.get("body");
            let resp = client
                .request_raw(&base_url, method, endpoint, body)
                .await?;
            let text = resp.text().await.unwrap_or_default();
            let val: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
            Ok(val)
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}
