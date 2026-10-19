use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::types::{AuthMethod, Config, IssueTemplate, Site, ViewQueries};
use super::view_mode::{self, ViewMode};
use crate::auth::Auth;

impl Config {
    pub fn custom_field_ids_for_fetch(&self) -> Vec<String> {
        let columns = crate::columns::Column::resolve(self.columns.as_deref());
        let base = crate::columns::Column::custom_field_ids(&columns);
        self.detail.merge_custom_field_ids(base)
    }

    pub fn config_dir() -> Result<PathBuf, String> {
        let base =
            dirs::config_dir().ok_or_else(|| "Cannot determine config directory".to_string())?;
        Ok(base.join("tick"))
    }

    pub fn config_path() -> Result<PathBuf, String> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn token_path() -> Result<PathBuf, String> {
        Ok(Self::config_dir()?.join("token"))
    }

    pub fn context_path() -> Result<PathBuf, String> {
        Ok(Self::config_dir()?.join("context"))
    }

    pub fn read_active_context() -> Option<String> {
        let path = Self::context_path().ok()?;
        if path.is_file() {
            let s = fs::read_to_string(&path).ok()?;
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    pub fn set_active_context(site_name: &str) -> Result<(), String> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;
        let path = Self::context_path()?;
        fs::write(&path, site_name.trim())
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        Ok(())
    }

    pub fn clear_active_context() -> Result<(), String> {
        let path = Self::context_path()?;
        if path.is_file() {
            fs::remove_file(&path)
                .map_err(|e| format!("Cannot remove {}: {}", path.display(), e))?;
        }
        Ok(())
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        let mut config: Config =
            toml::from_str(&contents).map_err(|e| format!("Invalid config: {}", e))?;
        if config.auth == AuthMethod::Token {
            config.token = Self::resolve_token(&config.token)?;
        }
        config.view_jql = Self::build_view_jql(&config.views);
        crate::template_export::merge_templates_file(&mut config)?;
        config.validate()?;
        Ok(config)
    }

    pub async fn resolve_auth(&self) -> Result<Auth, String> {
        match self.auth {
            AuthMethod::Token => {
                let token = Self::resolve_token(&self.token)?;
                Ok(Auth::basic(&self.email, &token))
            }
            AuthMethod::Oauth => crate::oauth::load_auth(&self.oauth).await,
        }
    }

    /// TICK_TOKEN env → ~/.config/tick/token file → config.toml `token`
    pub fn resolve_token(config_token: &str) -> Result<String, String> {
        if let Ok(t) = std::env::var("TICK_TOKEN") {
            let t = t.trim();
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
        let path = Self::token_path()?;
        if path.is_file() {
            let t = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            let t = t.trim();
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
        let t = config_token.trim();
        if t.is_empty() || t == "your-api-token" {
            return Err(
                "No API token: set TICK_TOKEN, create ~/.config/tick/token, or set token in config.toml"
                    .into(),
            );
        }
        Ok(t.to_string())
    }

    pub(crate) fn build_view_jql(views: &ViewQueries) -> HashMap<ViewMode, String> {
        ViewMode::all()
            .into_iter()
            .map(|mode| {
                let jql = match mode {
                    ViewMode::MyIssues => views.assigned.as_deref(),
                    ViewMode::Mentions => views.mentions.as_deref(),
                    ViewMode::Watching => views.watched.as_deref(),
                    ViewMode::Updated => views.updated.as_deref(),
                    ViewMode::Sprint => views.sprint.as_deref(),
                    ViewMode::ClosedSearch => views.closed.as_deref(),
                };
                let s = jql
                    .map(String::from)
                    .unwrap_or_else(|| mode.default_jql().to_string());
                (mode, s)
            })
            .collect()
    }

    pub fn jql_for(&self, mode: ViewMode) -> &str {
        self.view_jql
            .get(&mode)
            .map(String::as_str)
            .unwrap_or(mode.default_jql())
    }

    /// Base JQL for the Closed tab (without search text).
    pub fn closed_search_base(&self, ever_assigned: bool) -> &str {
        if ever_assigned {
            self.views
                .closed_history
                .as_deref()
                .unwrap_or(ViewMode::closed_search_base(true))
        } else {
            self.views
                .closed
                .as_deref()
                .unwrap_or(ViewMode::closed_search_base(false))
        }
    }

    pub fn build_closed_search_jql(&self, query: &str, ever_assigned: bool) -> String {
        let base = self.closed_search_base(ever_assigned);
        view_mode::build_closed_search_jql(base, query)
    }

    /// Sites to query for a fetch (`site_filter` = `[[sites]].name`).
    pub fn sites_for_fetch(&self, site_filter: Option<&str>) -> Vec<&Site> {
        match site_filter {
            Some(name) => self.sites.iter().filter(|s| s.name == name).collect(),
            None => self.sites.iter().collect(),
        }
    }

    /// Resolve tab keys for custom views (defaults 7, 8, 9).
    pub fn custom_view_keys(&self) -> Vec<(u8, usize)> {
        let mut used = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (i, view) in self.views.custom.iter().enumerate() {
            let key = view.key.unwrap_or(match i {
                0 => 7,
                1 => 8,
                _ => 9,
            });
            if (7..=9).contains(&key) && used.insert(key) {
                out.push((key, i));
            }
        }
        out
    }

    /// Apply CLI overrides and re-validate (call after `load()`).
    pub fn apply_cli_overrides(
        &mut self,
        max_results: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<(), String> {
        if let Some(mr) = max_results {
            self.max_results = mr;
        }
        if let Some(ps) = page_size {
            self.page_size = ps;
        }
        if max_results.is_some() || page_size.is_some() {
            self.validate()?;
        }
        Ok(())
    }

    /// Templates visible for a site (or all when `site_name` is None).
    pub fn issue_templates_for_site<'a>(
        &'a self,
        site_name: Option<&str>,
    ) -> Vec<&'a IssueTemplate> {
        self.create
            .templates
            .iter()
            .filter(|t| template_matches_site(t, site_name))
            .collect()
    }

    pub fn create_default_config() -> Result<(), String> {
        let path = Self::config_path()?;
        if path.exists() {
            return Ok(());
        }
        let parent = path
            .parent()
            .ok_or_else(|| "Config path has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create config dir {}: {}", parent.display(), e))?;
        let default = r#"email = "you@example.com"
# API token auth is the default (no auth line needed).
# token = "your-api-token"   # or use ~/.config/tick/token or TICK_TOKEN env
max_results = 50
page_size = 10   # rows to scroll with [ and ]
theme = "dark"   # minimal dark mode by default; options: "dark", "light", "catppuccin-mocha", etc.
# notify_on_refresh = true   # desktop alert when new issues appear on refresh
# auth = "oauth"             # optional; default is token — see docs/OAUTH.md
# [oauth]
# client_id = "your-atlassian-oauth-app-id"
# redirect_uri = "http://127.0.0.1:8765/callback"

# Optional custom JQL per view (defaults shown commented)
# [views]
# assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# mentions = "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
# watched = "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# updated = "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
# sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC"
# closed = "assignee = currentUser() AND statusCategory = Done"   # 6th tab — add text via /
# closed_history = "assignee was currentUser() AND statusCategory = Done"   # h toggles on Closed tab
#
# [[views.custom]]
# name = "My bugs"
# jql = "project = DEMO AND assignee = currentUser() ORDER BY updated DESC"
# key = 7   # tab key 7, 8, or 9

# Optional table columns (ids: site, key, type, status, priority, age, due, assignee, reporter, parent, labels, sprint, summary)
# columns = ["site", "key", "labels", "sprint", "summary", "status", "assignee"]

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
# sprint_field = "customfield_10020"   # run: tick --doctor
# board_id = 7                         # default board for sprint moves (M)
# boards = { DEMO = 7, WEB = 12 }      # per-project board overrides
# create_project = "ENG"
# create_issue_type = "Task"

# Issue templates — press N to create; X on a ticket exports it as a template
# Optional: templates_file = "templates/local.toml"  # merged at load
# [[create.templates]]
# name = "bug"
# site = "my-team"                    # required if you have multiple [[sites]]
# project = "ENG"
# issue_type = "Bug"
# summary = "Bug: "
# description = "Use triple-quotes in your real config for multiline text"
# labels = ["bug"]
# priority = "Medium"
"#;
        fs::write(&path, default).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        println!("Default config created at {}", path.display());
        Ok(())
    }
}

fn template_matches_site(template: &IssueTemplate, site_name: Option<&str>) -> bool {
    match (&template.site, site_name) {
        (None, _) => true,
        (Some(tmpl_site), Some(active)) => tmpl_site == active,
        (Some(_), None) => true,
    }
}
