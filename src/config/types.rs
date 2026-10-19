use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::view_mode::ViewMode;
use crate::api::agile::BoardConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    #[default]
    Token,
    Oauth,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OAuthSettings {
    #[serde(default)]
    pub client_id: String,
    #[serde(default = "default_oauth_redirect")]
    pub redirect_uri: String,
}

pub(crate) fn default_oauth_redirect() -> String {
    "http://127.0.0.1:8765/callback".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub base_url: String,
    /// Jira field id for sprint (e.g. customfield_10020). Use `tick --doctor` to discover.
    #[serde(default)]
    pub sprint_field: Option<String>,
    /// Default agile board id for sprint moves (`M`). See `tick --doctor`.
    #[serde(default)]
    pub board_id: Option<u64>,
    /// Per-project board overrides: `boards = { DEMO = 7, WEB = 12 }`
    #[serde(default)]
    pub boards: HashMap<String, u64>,
    /// Default project key for `n` (new issue).
    #[serde(default)]
    pub create_project: Option<String>,
    /// Default issue type name for `n`.
    #[serde(default)]
    pub create_issue_type: Option<String>,
    /// After duplicate, link new issue to source (`Cloners` by default).
    #[serde(default = "default_true")]
    pub create_clone_link: bool,
    /// Issue link type name for duplicate.
    #[serde(default = "default_clone_link_type")]
    pub clone_link_type: String,
    /// Issue link type names for add-link (`I`); see `SiteLinkTypes`.
    #[serde(default)]
    pub link_types: SiteLinkTypes,
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_clone_link_type() -> String {
    "Cloners".into()
}

pub(crate) fn default_link_relates() -> String {
    crate::api::issue_relations::DEFAULT_LINK_RELATES.into()
}

pub(crate) fn default_link_blocks() -> String {
    crate::api::issue_relations::DEFAULT_LINK_BLOCKS.into()
}

pub(crate) fn default_link_epic() -> String {
    crate::api::issue_relations::DEFAULT_LINK_EPIC.into()
}

/// Link type names per site for interactive add-link (`I`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteLinkTypes {
    #[serde(default = "default_link_relates")]
    pub relates: String,
    #[serde(default = "default_link_blocks")]
    pub blocks: String,
    #[serde(default = "default_link_blocks")]
    pub blocked_by: String,
    #[serde(default = "default_link_epic")]
    pub epic: String,
}

impl Default for SiteLinkTypes {
    fn default() -> Self {
        Self {
            relates: default_link_relates(),
            blocks: default_link_blocks(),
            blocked_by: default_link_blocks(),
            epic: default_link_epic(),
        }
    }
}

impl SiteLinkTypes {
    /// `(Jira type name, picker label)` — four entries, fixed order.
    pub fn picker_options(&self) -> Vec<(String, String)> {
        vec![
            (
                self.relates.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[0].into(),
            ),
            (
                self.blocks.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[1].into(),
            ),
            (
                self.blocked_by.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[2].into(),
            ),
            (
                self.epic.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[3].into(),
            ),
        ]
    }
}

impl Site {
    pub fn board_config(&self) -> BoardConfig {
        BoardConfig {
            default_board_id: self.board_id,
            project_boards: self.boards.clone(),
        }
    }

    pub fn is_board_configured(&self, board_id: u64, project_key: Option<&str>) -> bool {
        if self.board_id == Some(board_id) {
            return true;
        }
        if let Some(pk) = project_key {
            return self.boards.get(pk) == Some(&board_id);
        }
        false
    }

    pub fn clone_link_enabled(&self) -> bool {
        self.create_clone_link
    }

    pub fn clone_link_type_name(&self) -> &str {
        if self.clone_link_type.is_empty() {
            "Cloners"
        } else {
            &self.clone_link_type
        }
    }
}

/// Saved JQL view — switch with configured key (`7`–`9`) or `v` / `Shift+V`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomView {
    pub name: String,
    pub jql: String,
    /// Query only this site (must match `[[sites]].name`).
    #[serde(default)]
    pub site: Option<String>,
    /// Tab key `7`, `8`, or `9` (auto-assigned to 7, 8, 9 for first entries).
    #[serde(default)]
    pub key: Option<u8>,
}

impl CustomView {
    pub fn cache_slug(&self) -> String {
        self.name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ViewQueries {
    pub assigned: Option<String>,
    pub updated: Option<String>,
    pub mentions: Option<String>,
    pub watched: Option<String>,
    pub sprint: Option<String>,
    /// Closed search: assignee when done (base JQL without `text ~` or ORDER BY).
    pub closed: Option<String>,
    /// Closed search: ever assigned to you (`assignee was`).
    pub closed_history: Option<String>,
    #[serde(default)]
    pub custom: Vec<CustomView>,
}

/// Pre-filled issue for `N` (create from template). Minimal edits: summary, then Enter.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct IssueTemplate {
    /// Short id shown in the template picker (unique in config).
    pub name: String,
    /// Limit template to one site; required when you have multiple `[[sites]]`.
    pub site: Option<String>,
    pub project: String,
    #[serde(rename = "issue_type")]
    pub issue_type: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub priority: Option<String>,
    /// Jira account id (not display name).
    pub assignee_account_id: Option<String>,
    pub parent_key: Option<String>,
    /// Extra create fields by id, e.g. `customfield_10001 = "value"`.
    #[serde(default)]
    pub extra_fields: HashMap<String, toml::Value>,
}

impl IssueTemplate {
    pub fn validate_fields(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Template name cannot be empty".into());
        }
        if self.project.trim().is_empty() {
            return Err("Template needs a project".into());
        }
        if self.issue_type.trim().is_empty() {
            return Err("Template needs an issue type".into());
        }
        if self.summary.trim().is_empty() {
            return Err("Template needs a summary".into());
        }
        Ok(())
    }
}

/// Shell command run after a successful view refresh (`[[hooks.on_refresh]]`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefreshHook {
    pub command: String,
    /// View ids: `assigned`, `mentions`, `watched`, `updated`, `sprint`, `closed`, or custom view name.
    #[serde(default)]
    pub views: Vec<String>,
    #[serde(default = "default_hook_timeout_secs")]
    pub timeout_secs: u64,
}

pub(crate) fn default_hook_timeout_secs() -> u64 {
    30
}

/// Shell command run after TUI or CLI bulk actions finish (`[[hooks.on_bulk_complete]]`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BulkCompleteHook {
    pub command: String,
    #[serde(default = "default_hook_timeout_secs")]
    pub timeout_secs: u64,
}

/// Shell command run after `R` reloads `config.toml` (`[[hooks.on_config_reload]]`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigReloadHook {
    pub command: String,
    #[serde(default = "default_hook_timeout_secs")]
    pub timeout_secs: u64,
}

/// Shell command when a row is bulk-marked with Space (`[[hooks.on_mark]]`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarkHook {
    pub command: String,
    #[serde(default = "default_hook_timeout_secs")]
    pub timeout_secs: u64,
}

/// Detail pane settings (`[detail]` and `[[detail.editable_fields]]`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DetailSettings {
    #[serde(default, rename = "editable_fields")]
    pub editable_fields: Vec<EditableFieldConfig>,
}

/// Configured custom field editable from the issue detail pane (phase 1: text, select, user).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditableFieldConfig {
    /// Jira field id, e.g. `customfield_10042`.
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type", default = "default_editable_field_type")]
    pub field_type: String,
    /// Required when `type = "select"`: allowed option labels/values.
    #[serde(default)]
    pub options: Vec<String>,
}

pub(crate) fn default_editable_field_type() -> String {
    "text".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableFieldKind {
    Text,
    Select,
    User,
    Number,
    Date,
    DateTime,
    Boolean,
    MultiSelect,
    /// Resolve type and options from Jira edit metadata for the selected issue.
    Auto,
}

impl EditableFieldConfig {
    pub fn display_label(&self) -> String {
        self.label
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn parsed_kind(&self) -> Result<EditableFieldKind, String> {
        match self.field_type.trim().to_ascii_lowercase().as_str() {
            "text" | "string" => Ok(EditableFieldKind::Text),
            "select" | "option" => Ok(EditableFieldKind::Select),
            "user" => Ok(EditableFieldKind::User),
            "number" | "float" => Ok(EditableFieldKind::Number),
            "date" => Ok(EditableFieldKind::Date),
            "datetime" => Ok(EditableFieldKind::DateTime),
            "boolean" | "bool" => Ok(EditableFieldKind::Boolean),
            "multiselect" | "multi" => Ok(EditableFieldKind::MultiSelect),
            "auto" => Ok(EditableFieldKind::Auto),
            other => Err(format!("unknown editable field type '{other}'")),
        }
    }
}

impl DetailSettings {
    /// Union of table column custom fields and configured editable fields (for Jira fetch).
    pub fn merge_custom_field_ids(&self, column_ids: Vec<String>) -> Vec<String> {
        let mut ids = column_ids;
        for field in &self.editable_fields {
            if !ids.iter().any(|id| id == &field.id) {
                ids.push(field.id.clone());
            }
        }
        ids
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HooksSettings {
    #[serde(default, rename = "on_refresh")]
    pub on_refresh: Vec<RefreshHook>,
    #[serde(default, rename = "on_bulk_complete")]
    pub on_bulk_complete: Vec<BulkCompleteHook>,
    #[serde(default, rename = "on_config_reload")]
    pub on_config_reload: Vec<ConfigReloadHook>,
    #[serde(default, rename = "on_mark")]
    pub on_mark: Vec<MarkHook>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CreateSettings {
    #[serde(default = "default_clone_summary_prefix")]
    pub clone_summary_prefix: String,
    /// Optional file under the config dir (e.g. `templates/local.toml`) merged at load.
    #[serde(default)]
    pub templates_file: Option<String>,
    #[serde(default)]
    pub templates: Vec<IssueTemplate>,
}

pub(crate) fn default_clone_summary_prefix() -> String {
    "Copy of: ".into()
}

pub(crate) fn default_notice_secs() -> u64 {
    5
}

pub(crate) fn default_theme() -> String {
    "dark".to_string()
}

pub(crate) fn default_max_results() -> u32 {
    50
}

pub(crate) fn default_page_size() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub email: String,
    #[serde(default)]
    pub token: String,
    pub sites: Vec<Site>,
    /// Optional default site name from sites list
    #[serde(default)]
    pub default_site: Option<String>,
    #[serde(default)]
    pub columns: Option<Vec<String>>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub views: ViewQueries,
    /// Desktop notification when a background or scheduled refresh finds new issues.
    #[serde(default)]
    pub notify_on_refresh: bool,
    #[serde(default)]
    pub auth: AuthMethod,
    #[serde(default)]
    pub oauth: OAuthSettings,
    #[serde(default)]
    pub create: CreateSettings,
    #[serde(default)]
    pub hooks: HooksSettings,
    #[serde(default)]
    pub detail: DetailSettings,
    /// How long footer success notices stay visible (seconds).
    #[serde(default = "default_notice_secs")]
    pub notice_secs: u64,
    #[serde(skip)]
    pub view_jql: HashMap<ViewMode, String>,
}
