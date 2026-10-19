use serde::Serialize;

use crate::api::types::Ticket;
use crate::config::{Config, Site};
use crate::issue_key;

#[derive(Serialize)]
pub struct IssueJson {
    pub key: String,
    pub site: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub labels: Vec<String>,
    pub url: String,
}

impl IssueJson {
    pub fn from_ticket(ticket: &Ticket, site_name: &str) -> Self {
        Self {
            key: ticket.key.clone(),
            site: site_name.to_string(),
            summary: ticket.summary.clone(),
            status: ticket.status.clone(),
            priority: ticket.priority.clone(),
            assignee: ticket.assignee.clone(),
            labels: ticket.labels.clone(),
            url: ticket.link.clone(),
        }
    }
}

pub fn parse_issue_key_arg(raw: &str) -> Result<String, String> {
    issue_key::parse_issue_key(raw).ok_or_else(|| format!("Invalid issue key: {raw}"))
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveContextInfo {
    pub name: String,
    pub source: &'static str,
    pub base_url: String,
}

pub fn current_context(config: &Config) -> Option<ActiveContextInfo> {
    if let Ok(name) = std::env::var("TICK_SITE").or_else(|_| std::env::var("TICK_CONTEXT")) {
        let name = name.trim();
        if !name.is_empty() {
            if let Some(site) = config.sites.iter().find(|s| s.name == name) {
                return Some(ActiveContextInfo {
                    name: site.name.clone(),
                    source: "environment variable",
                    base_url: site.base_url.clone(),
                });
            }
        }
    }
    if let Some(name) = Config::read_active_context() {
        let name = name.trim();
        if !name.is_empty() {
            if let Some(site) = config.sites.iter().find(|s| s.name == name) {
                return Some(ActiveContextInfo {
                    name: site.name.clone(),
                    source: "context file (~/.config/tick/context)",
                    base_url: site.base_url.clone(),
                });
            }
        }
    }
    if let Some(ref name) = config.default_site {
        let name = name.trim();
        if !name.is_empty() {
            if let Some(site) = config.sites.iter().find(|s| s.name == name) {
                return Some(ActiveContextInfo {
                    name: site.name.clone(),
                    source: "config default_site",
                    base_url: site.base_url.clone(),
                });
            }
        }
    }
    if config.sites.len() == 1 {
        return Some(ActiveContextInfo {
            name: config.sites[0].name.clone(),
            source: "default (single configured site)",
            base_url: config.sites[0].base_url.clone(),
        });
    }
    None
}

pub fn resolve_site_for_command<'a>(
    config: &'a Config,
    site_arg: Option<&str>,
) -> Result<&'a Site, String> {
    if let Some(name) = site_arg {
        return config
            .sites
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("Unknown site '{name}'"));
    }
    if let Ok(name) = std::env::var("TICK_SITE").or_else(|_| std::env::var("TICK_CONTEXT")) {
        let name = name.trim();
        if !name.is_empty() {
            return config
                .sites
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| format!("Unknown site '{name}' from TICK_SITE/TICK_CONTEXT"));
        }
    }
    if let Some(name) = Config::read_active_context() {
        let name = name.trim();
        if !name.is_empty() {
            if let Some(site) = config.sites.iter().find(|s| s.name == name) {
                return Ok(site);
            }
        }
    }
    if let Some(ref name) = config.default_site {
        let name = name.trim();
        if !name.is_empty() {
            if let Some(site) = config.sites.iter().find(|s| s.name == name) {
                return Ok(site);
            }
        }
    }
    if config.sites.len() == 1 {
        return Ok(&config.sites[0]);
    }
    if config.sites.is_empty() {
        return Err(
            "No sites configured in config.toml. Run 'tick --init' to set up a site.".into(),
        );
    }
    Err("Multiple sites configured. Set active server context with 'tick context use <site>' or pass --site <name>".into())
}

pub fn resolve_site<'a>(
    config: &'a Config,
    key: &str,
    site_arg: Option<&str>,
) -> Result<&'a Site, String> {
    resolve_site_for_command(config, site_arg).map_err(|e| {
        if e.starts_with("Multiple sites") {
            format!("{e} for {key}")
        } else {
            e
        }
    })
}

pub fn require_site<'a>(config: &'a Config, name: &str) -> Result<&'a Site, String> {
    config
        .sites
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("Unknown site '{name}'"))
}

/// `--body` argument or stdin when omitted (for piped comments).
pub fn read_body_arg(body: Option<String>) -> Result<String, String> {
    if let Some(b) = body {
        if b.trim().is_empty() {
            return Err("comment body must not be empty".into());
        }
        return Ok(b);
    }
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("read stdin: {e}"))?;
    if buf.trim().is_empty() {
        return Err("comment body required: pass --body or pipe text on stdin".into());
    }
    Ok(buf)
}

pub fn parse_keys_list(keys: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for k in keys {
        for part in k.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            out.push(parse_issue_key_arg(part)?);
        }
    }
    if out.is_empty() {
        return Err("Provide at least one issue key (--keys)".into());
    }
    Ok(out)
}

pub fn parse_labels_input(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Site;

    fn test_config_multi() -> Config {
        Config {
            email: "test@example.com".into(),
            token: "tok".into(),
            sites: vec![
                Site {
                    name: "site-a".into(),
                    base_url: "https://site-a.atlassian.net".into(),
                    ..Default::default()
                },
                Site {
                    name: "site-b".into(),
                    base_url: "https://site-b.atlassian.net".into(),
                    ..Default::default()
                },
            ],
            default_site: None,
            columns: None,
            max_results: 50,
            page_size: 10,
            theme: "dark".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            create: Default::default(),
            hooks: Default::default(),
            detail: Default::default(),
            notice_secs: 5,
            view_jql: Default::default(),
        }
    }

    #[test]
    fn resolve_site_with_explicit_arg() {
        let config = test_config_multi();
        let site = resolve_site_for_command(&config, Some("site-b")).unwrap();
        assert_eq!(site.name, "site-b");
    }

    #[test]
    fn resolve_site_unknown_arg_err() {
        let config = test_config_multi();
        let err = resolve_site_for_command(&config, Some("non-existent")).unwrap_err();
        assert!(err.contains("Unknown site 'non-existent'"));
    }

    #[test]
    fn resolve_site_single_site_auto_resolves() {
        let mut config = test_config_multi();
        config.sites.truncate(1);
        let site = resolve_site_for_command(&config, None).unwrap();
        assert_eq!(site.name, "site-a");
    }

    #[test]
    fn resolve_site_default_site_config() {
        let mut config = test_config_multi();
        config.default_site = Some("site-b".into());
        let site = resolve_site_for_command(&config, None).unwrap();
        assert_eq!(site.name, "site-b");
    }

    #[test]
    fn resolve_site_multi_site_no_context_err() {
        let config = test_config_multi();
        let err = resolve_site_for_command(&config, None).unwrap_err();
        assert!(err.contains("Multiple sites configured"));
    }
}
