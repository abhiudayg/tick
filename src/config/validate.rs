use std::collections::HashSet;

use super::types::{AuthMethod, Config, EditableFieldKind};

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("config: email must not be empty".into());
        }
        if self.auth == AuthMethod::Oauth {
            let path = crate::oauth::token_path()?;
            if !path.exists() {
                return Err("OAuth not configured: run `tick auth login`".into());
            }
        }
        if self.page_size == 0 {
            return Err("config: page_size must be at least 1".into());
        }
        if self.page_size > 500 {
            return Err("config: page_size must be at most 500".into());
        }
        if self.sites.is_empty() {
            return Err("config: at least one [[sites]] entry is required".into());
        }

        let mut template_names = HashSet::new();
        for template in &self.create.templates {
            let name = template.name.trim();
            if name.is_empty() {
                return Err("config: create.templates name must not be empty".into());
            }
            if !template_names.insert(name.to_string()) {
                return Err(format!("config: duplicate template name '{name}'"));
            }
            if template.project.trim().is_empty() {
                return Err(format!("config: template '{name}' needs project"));
            }
            if template.issue_type.trim().is_empty() {
                return Err(format!("config: template '{name}' needs issue_type"));
            }
            if template.summary.trim().is_empty() {
                return Err(format!("config: template '{name}' needs summary"));
            }
            if let Some(ref site) = template.site {
                if !self.sites.iter().any(|s| s.name == *site) {
                    return Err(format!(
                        "config: template '{name}' references unknown site '{site}'"
                    ));
                }
            } else if self.sites.len() > 1 {
                return Err(format!(
                    "config: template '{name}' needs site = \"...\" when using multiple [[sites]]"
                ));
            }
        }

        let mut custom_names = HashSet::new();
        let mut custom_keys = HashSet::new();
        for view in &self.views.custom {
            let name = view.name.trim();
            if name.is_empty() {
                return Err("config: views.custom name must not be empty".into());
            }
            if !custom_names.insert(name.to_string()) {
                return Err(format!("config: duplicate custom view '{name}'"));
            }
            if view.jql.trim().is_empty() {
                return Err(format!("config: custom view '{name}' needs jql"));
            }
            if let Some(ref site) = view.site {
                if !self.sites.iter().any(|s| s.name == *site) {
                    return Err(format!(
                        "config: custom view '{name}' references unknown site '{site}'"
                    ));
                }
            }
            if let Some(key) = view.key {
                if !(7..=9).contains(&key) {
                    return Err(format!(
                        "config: custom view '{name}' key must be 7, 8, or 9 (got {key})"
                    ));
                }
                if !custom_keys.insert(key) {
                    return Err(format!("config: duplicate custom view key {key}"));
                }
            }
        }

        for site in &self.sites {
            if site.name.trim().is_empty() {
                return Err("config: site name must not be empty".into());
            }
            let url = site.base_url.trim();
            if url.is_empty() {
                return Err(format!(
                    "config: base_url for site '{}' must not be empty",
                    site.name
                ));
            }
            if !url.starts_with("https://") {
                return Err(format!(
                    "config: base_url for site '{}' must start with https://",
                    site.name
                ));
            }
        }

        let mut editable_ids = HashSet::new();
        for field in &self.detail.editable_fields {
            let id = field.id.trim();
            if id.is_empty() {
                return Err("config: detail.editable_fields id must not be empty".into());
            }
            if !id.starts_with("customfield_") {
                return Err(format!(
                    "config: editable field '{id}' id must be customfield_<digits>"
                ));
            }
            let suffix = id.strip_prefix("customfield_").unwrap_or("");
            if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("config: editable field '{id}' has invalid id"));
            }
            if !editable_ids.insert(id.to_string()) {
                return Err(format!("config: duplicate editable field '{id}'"));
            }
            let kind = field.parsed_kind().map_err(|e| format!("config: {e}"))?;
            if kind == EditableFieldKind::Select && field.options.is_empty() {
                return Err(format!(
                    "config: editable field '{}' (select) needs options = [...]",
                    field.display_label()
                ));
            }
            if kind == EditableFieldKind::MultiSelect && field.options.is_empty() {
                return Err(format!(
                    "config: editable field '{}' (multiselect) needs options = [...] or type = \"auto\"",
                    field.display_label()
                ));
            }
        }

        Ok(())
    }
}
