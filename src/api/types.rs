use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedView {
    pub fetched_at: String,
    pub tickets: Vec<Ticket>,
}

/// A Jira workflow transition (the only supported way to change issue status).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTransition {
    pub id: String,
    /// Workflow action label (e.g. "Start Progress").
    pub name: String,
    /// Target status after the transition (e.g. "In Progress").
    pub to_status: String,
    /// Fields Jira marks as required for this transition (from `transitions.fields`).
    pub required_fields: Vec<super::transition_fields::TransitionField>,
}

impl WorkflowTransition {
    pub fn label(&self) -> String {
        if self.to_status.is_empty() {
            self.name.clone()
        } else if self.name.eq_ignore_ascii_case(&self.to_status) {
            self.to_status.clone()
        } else {
            format!("{} → {}", self.name, self.to_status)
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ticket {
    pub key: String,
    pub site: String,
    pub issue_type: String,
    pub status: String,
    pub status_color: String,
    pub priority: String,
    pub ageing_days: i64,
    pub due_date: Option<NaiveDate>,
    pub assignee: String,
    pub reporter: String,
    pub summary: String,
    pub link: String,
    pub description: Option<String>,
    pub description_adf: Option<serde_json::Value>,
    pub latest_comment: Option<String>,
    pub all_comments: Vec<CommentEntry>,
    pub parent_key: Option<String>,
    pub parent_summary: Option<String>,
    pub labels: Vec<String>,
    pub sprint_name: Option<String>,
    #[serde(default)]
    pub project_key: String,
    /// Read-only custom field values keyed by field id (e.g. `customfield_10042`).
    #[serde(default)]
    pub custom_fields: HashMap<String, String>,
    /// Description and comments were fetched (lazy detail load).
    #[serde(default)]
    pub detail_loaded: bool,
    /// Set when lazy detail fetch failed (avoids infinite "Loading…").
    #[serde(default, skip_serializing)]
    pub detail_error: Option<String>,
}

pub(crate) fn project_key_from_issue_key(key: &str) -> &str {
    key.rfind('-').map_or(key, |i| &key[..i])
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommentEntry {
    pub author: String,
    pub created: String,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct JqlSearchResponse {
    pub issues: Vec<JqlIssue>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
    #[serde(rename = "isLast", default)]
    pub is_last: bool,
}

#[derive(Debug, Deserialize)]
pub struct JqlIssue {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkFetchResponse {
    pub issues: Vec<BulkFetchIssue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkFetchIssue {
    pub key: String,
    pub fields: JiraFields,
}

/// Subset of issue fields loaded on demand when the detail pane opens.
#[derive(Debug, Deserialize, Serialize)]
pub struct IssueDetailFields {
    pub description: Option<serde_json::Value>,
    pub comment: Option<JiraComments>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct IssueDetailResponse {
    pub fields: IssueDetailFields,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraFields {
    #[serde(rename = "issuetype")]
    pub issue_type: JiraNamed,
    pub status: JiraStatus,
    pub priority: Option<JiraNamed>,
    pub assignee: Option<JiraUser>,
    pub reporter: Option<JiraUser>,
    pub duedate: Option<NaiveDate>,
    pub created: String,
    #[allow(dead_code)]
    pub project: JiraProject,
    pub summary: String,
    pub description: Option<serde_json::Value>,
    pub comment: Option<JiraComments>,
    pub parent: Option<JiraParent>,
    pub labels: Option<Vec<String>>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraStatus {
    pub name: String,
    #[serde(rename = "statusCategory")]
    pub status_category: JiraStatusCategory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraStatusCategory {
    #[allow(dead_code)]
    pub key: String,
    #[serde(rename = "colorName")]
    pub color_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JiraComments {
    pub comments: Vec<JiraComment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JiraComment {
    pub body: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub author: Option<JiraUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JiraNamed {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JiraUser {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraProject {
    #[allow(dead_code)]
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraParent {
    pub key: String,
    pub fields: JiraParentFields,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraParentFields {
    pub summary: String,
}

pub(crate) fn extract_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(m) => {
            let node_type = m.get("type").and_then(|t| t.as_str());
            if node_type == Some("mention") {
                return m
                    .get("attrs")
                    .and_then(|a| a.get("text"))
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
            }
            if node_type == Some("hardBreak") {
                return Some("\n".into());
            }
            if let Some(content) = m.get("content").and_then(|c| c.as_array()) {
                let sep = match node_type {
                    Some("paragraph") => "",
                    _ => "\n",
                };
                let mut parts = Vec::new();
                for node in content {
                    if let Some(text) = extract_text(node) {
                        parts.push(text);
                    }
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(sep))
                }
            } else {
                m.get("text")
                    .and_then(|t| t.as_str())
                    .map(|text| text.to_string())
            }
        }
        _ => None,
    }
}

/// Collect `@mention` labels from an ADF document (description or comment body).
pub fn collect_mention_labels(doc: &serde_json::Value) -> Vec<String> {
    let mut labels: Vec<String> = collect_mentions(doc)
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    labels.sort();
    labels.dedup();
    labels
}

/// Collect `(display label, accountId)` for every ADF mention node.
pub fn collect_mentions(doc: &serde_json::Value) -> Vec<(String, String)> {
    let mut mentions = Vec::new();
    collect_mentions_rec(doc, &mut mentions);
    mentions
}

fn collect_mentions_rec(node: &serde_json::Value, out: &mut Vec<(String, String)>) {
    if let Some(obj) = node.as_object() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("mention") {
            let attrs = obj.get("attrs");
            let account_id = attrs
                .and_then(|a| a.get("id"))
                .and_then(|id| id.as_str())
                .unwrap_or("");
            let label = attrs
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("@user");
            if !account_id.is_empty() || !label.is_empty() {
                out.push((label.to_string(), account_id.to_string()));
            }
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
            for child in content {
                collect_mentions_rec(child, out);
            }
        }
    }
}

pub(crate) fn extract_sprint_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Object(m) => m
            .get("name")
            .and_then(|n| n.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from),
        serde_json::Value::Array(arr) => arr.iter().find_map(extract_sprint_name),
        _ => None,
    }
}

fn comments_from_jira(comment: Option<&JiraComments>) -> Vec<CommentEntry> {
    comment
        .map(|c| {
            c.comments
                .iter()
                .map(|cmt| CommentEntry {
                    author: cmt
                        .author
                        .as_ref()
                        .map(|a| a.display_name.clone())
                        .unwrap_or_default(),
                    created: cmt.created.clone(),
                    body: cmt.body.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

impl Ticket {
    pub fn project_key_for_api(&self) -> &str {
        if !self.project_key.is_empty() {
            &self.project_key
        } else {
            project_key_from_issue_key(&self.key)
        }
    }

    pub fn clear_detail_fields(&mut self) {
        self.description = None;
        self.description_adf = None;
        self.latest_comment = None;
        self.all_comments.clear();
        self.detail_loaded = false;
        self.detail_error = None;
    }

    /// Mark lazy detail as finished after a failed fetch (stops perpetual loading UI).
    pub fn mark_detail_failed(&mut self, message: impl Into<String>) {
        self.clear_detail_fields();
        self.detail_loaded = true;
        self.detail_error = Some(message.into());
    }

    pub fn apply_detail_fields(&mut self, fields: &IssueDetailFields) {
        self.detail_error = None;
        self.description_adf = fields.description.clone();
        self.description = self.description_adf.as_ref().and_then(extract_text);
        self.all_comments = comments_from_jira(fields.comment.as_ref());
        self.latest_comment = self
            .all_comments
            .last()
            .and_then(|c| c.body.as_ref())
            .and_then(extract_text);
        self.detail_loaded = true;
    }

    pub fn from_bulk_fetch(
        issue: BulkFetchIssue,
        site_name: &str,
        base_url: &str,
        sprint_field: Option<&str>,
        custom_field_ids: &[&str],
        include_detail: bool,
    ) -> Self {
        let ageing_days = NaiveDate::parse_from_str(&issue.fields.created[..10], "%Y-%m-%d")
            .map(|d| (chrono::Utc::now().date_naive() - d).num_days())
            .unwrap_or(0);

        let mut ticket = Self {
            key: issue.key.clone(),
            site: site_name.to_string(),
            issue_type: issue.fields.issue_type.name,
            status: issue.fields.status.name,
            status_color: issue.fields.status.status_category.color_name,
            priority: issue
                .fields
                .priority
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            ageing_days,
            due_date: issue.fields.duedate,
            assignee: issue
                .fields
                .assignee
                .as_ref()
                .map(|u| u.display_name.clone())
                .unwrap_or_default(),
            reporter: issue
                .fields
                .reporter
                .as_ref()
                .map(|u| u.display_name.clone())
                .unwrap_or_default(),
            summary: issue.fields.summary,
            link: format!("{}/browse/{}", base_url.trim_end_matches('/'), issue.key),
            description: None,
            description_adf: None,
            latest_comment: None,
            all_comments: Vec::new(),
            parent_key: issue.fields.parent.as_ref().map(|p| p.key.clone()),
            parent_summary: issue
                .fields
                .parent
                .as_ref()
                .map(|p| p.fields.summary.clone()),
            labels: issue.fields.labels.clone().unwrap_or_default(),
            sprint_name: sprint_field
                .and_then(|field| issue.fields.custom.get(field).and_then(extract_sprint_name)),
            project_key: issue.fields.project.key.clone(),
            custom_fields: custom_field_ids
                .iter()
                .filter_map(|id| {
                    issue
                        .fields
                        .custom
                        .get(*id)
                        .map(|v| (id.to_string(), format_custom_field_value(v)))
                })
                .collect(),
            detail_loaded: false,
            detail_error: None,
        };

        if include_detail {
            ticket.apply_detail_fields(&IssueDetailFields {
                description: issue.fields.description.clone(),
                comment: issue.fields.comment.clone(),
            });
        }

        ticket
    }
}

/// Display string for a Jira custom field JSON value (read-only table column).
pub fn format_custom_field_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::String(s) if s.is_empty() => "-".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(format_custom_field_value).collect();
            let joined = parts
                .into_iter()
                .filter(|p| p != "-")
                .collect::<Vec<_>>()
                .join(", ");
            if joined.is_empty() {
                "-".to_string()
            } else {
                joined
            }
        }
        serde_json::Value::Object(m) => m
            .get("name")
            .or_else(|| m.get("value"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .unwrap_or_else(|| "-".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn project_key_from_issue_key_parses_prefix() {
        assert_eq!(project_key_from_issue_key("DEMO-1"), "DEMO");
        assert_eq!(project_key_from_issue_key("MY-PROJ-42"), "MY-PROJ");
    }

    #[test]
    fn extract_text_from_plain_string() {
        let v = serde_json::json!("hello");
        assert_eq!(extract_text(&v), Some("hello".into()));
    }

    #[test]
    fn extract_text_from_adf_paragraph() {
        let v = serde_json::json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "line one"}]}]
        });
        assert_eq!(extract_text(&v), Some("line one".into()));
    }

    #[test]
    fn collect_mention_labels_dedupes() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "mention", "attrs": {"text": "@Ada"}},
                    {"type": "mention", "attrs": {"text": "@Ada"}},
                    {"type": "mention", "attrs": {"text": "@Bob"}},
                ]
            }]
        });
        assert_eq!(
            collect_mention_labels(&doc),
            vec!["@Ada".to_string(), "@Bob".to_string()]
        );
    }

    #[test]
    fn extract_text_includes_mention_label() {
        let v = serde_json::json!({
            "type": "paragraph",
            "content": [
                {"type": "text", "text": "hi "},
                {"type": "mention", "attrs": {"id": "1", "text": "@Ada"}},
                {"type": "text", "text": "!"}
            ]
        });
        assert_eq!(extract_text(&v), Some("hi @Ada!".into()));
    }

    #[test]
    fn jql_search_response_parses_issue_ids() {
        let raw = r#"{"issues":[{"id":"10001"},{"id":"10002"}]}"#;
        let data: JqlSearchResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(data.issues.len(), 2);
        assert_eq!(data.issues[0].id, "10001");
    }

    #[test]
    fn jql_search_response_parses_next_page_token() {
        let raw = r#"{"issues":[],"nextPageToken":"tok","isLast":false}"#;
        let data: JqlSearchResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(data.next_page_token.as_deref(), Some("tok"));
        assert!(!data.is_last);
    }

    #[test]
    fn ticket_from_bulk_fetch_maps_fields() {
        let issue = BulkFetchIssue {
            key: "PROJ-1".into(),
            fields: JiraFields {
                issue_type: JiraNamed { name: "Bug".into() },
                status: JiraStatus {
                    name: "In Progress".into(),
                    status_category: JiraStatusCategory {
                        key: "indeterminate".into(),
                        color_name: "yellow".into(),
                    },
                },
                priority: Some(JiraNamed {
                    name: "High".into(),
                }),
                assignee: Some(JiraUser {
                    display_name: "Alice".into(),
                }),
                reporter: Some(JiraUser {
                    display_name: "Bob".into(),
                }),
                duedate: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
                created: "2026-01-15T10:00:00.000+0000".into(),
                project: JiraProject { key: "PROJ".into() },
                summary: "Fix login".into(),
                description: Some(serde_json::json!("Plain description")),
                comment: None,
                parent: None,
                labels: Some(vec!["backend".into(), "urgent".into()]),
                custom: HashMap::new(),
            },
        };
        let ticket =
            Ticket::from_bulk_fetch(issue, "acme", "https://acme.atlassian.net", None, &[], true);
        assert_eq!(ticket.key, "PROJ-1");
        assert_eq!(ticket.site, "acme");
        assert_eq!(ticket.status, "In Progress");
        assert_eq!(ticket.assignee, "Alice");
        assert_eq!(ticket.description.as_deref(), Some("Plain description"));
        assert!(ticket.detail_loaded);
        assert!(ticket.link.contains("PROJ-1"));
        assert_eq!(ticket.labels, vec!["backend", "urgent"]);
    }

    #[test]
    fn extract_sprint_from_object_and_array() {
        let obj = serde_json::json!({ "name": "Sprint 1" });
        assert_eq!(extract_sprint_name(&obj), Some("Sprint 1".into()));
        let arr = serde_json::json!([{ "name": "Sprint 2" }]);
        assert_eq!(extract_sprint_name(&arr), Some("Sprint 2".into()));
    }

    #[test]
    fn ticket_maps_custom_sprint_field() {
        let mut custom = HashMap::new();
        custom.insert(
            "customfield_10020".into(),
            serde_json::json!({ "name": "Board Sprint" }),
        );
        let issue = BulkFetchIssue {
            key: "X-1".into(),
            fields: JiraFields {
                issue_type: JiraNamed {
                    name: "Task".into(),
                },
                status: JiraStatus {
                    name: "Open".into(),
                    status_category: JiraStatusCategory {
                        key: "new".into(),
                        color_name: "blue".into(),
                    },
                },
                priority: None,
                assignee: None,
                reporter: None,
                duedate: None,
                created: "2026-01-01T00:00:00.000+0000".into(),
                project: JiraProject { key: "X".into() },
                summary: "S".into(),
                description: None,
                comment: None,
                parent: None,
                labels: None,
                custom,
            },
        };
        let ticket = Ticket::from_bulk_fetch(
            issue,
            "acme",
            "https://acme.atlassian.net",
            Some("customfield_10020"),
            &["customfield_10020"],
            false,
        );
        assert_eq!(ticket.sprint_name.as_deref(), Some("Board Sprint"));
        assert!(!ticket.detail_loaded);
    }

    #[test]
    fn from_bulk_fetch_without_detail_omits_description() {
        let issue = BulkFetchIssue {
            key: "PROJ-2".into(),
            fields: JiraFields {
                issue_type: JiraNamed {
                    name: "Task".into(),
                },
                status: JiraStatus {
                    name: "Open".into(),
                    status_category: JiraStatusCategory {
                        key: "new".into(),
                        color_name: "blue".into(),
                    },
                },
                priority: None,
                assignee: None,
                reporter: None,
                duedate: None,
                created: "2026-01-01T00:00:00.000+0000".into(),
                project: JiraProject { key: "PROJ".into() },
                summary: "S".into(),
                description: Some(serde_json::json!("hidden until detail open")),
                comment: None,
                parent: None,
                labels: None,
                custom: HashMap::new(),
            },
        };
        let ticket = Ticket::from_bulk_fetch(
            issue,
            "acme",
            "https://acme.atlassian.net",
            None,
            &[],
            false,
        );
        assert!(ticket.description.is_none());
        assert!(!ticket.detail_loaded);
    }
}
