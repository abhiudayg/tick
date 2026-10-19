use super::*;
use crate::view_mode::ViewMode;

fn sample_config_toml(token: &str) -> String {
    format!(
        r#"
email = "a@b.com"
token = "{token}"
max_results = 25
theme = "dracula"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#
    )
}

#[test]
fn auth_defaults_to_token_when_omitted() {
    let toml = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "s"
base_url = "https://x.atlassian.net"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.auth, AuthMethod::Token);
}

#[test]
fn parses_minimal_config() {
    let cfg: Config = toml::from_str(&sample_config_toml("secret")).unwrap();
    let mut cfg = cfg;
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.validate().is_ok());
}

#[test]
fn custom_view_jql() {
    let raw = r#"
email = "a@b.com"
token = "secret"

[views]
assigned = "project = DEMO ORDER BY created DESC"
sprint = "sprint in openSprints() ORDER BY rank"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.jql_for(ViewMode::MyIssues).contains("DEMO"));
    assert!(cfg.jql_for(ViewMode::Sprint).contains("openSprints"));
}

#[test]
fn rejects_empty_sites() {
    let raw = r#"
email = "a@b.com"
token = "secret"
sites = []
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.validate().is_err());
}

#[test]
fn parses_board_config() {
    let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
board_id = 7
boards = { DEMO = 12, WEB = 15 }
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    let site = &cfg.sites[0];
    assert_eq!(site.board_id, Some(7));
    assert_eq!(site.boards.get("DEMO"), Some(&12));
    assert_eq!(site.board_config().project_boards.get("WEB"), Some(&15));
}

#[test]
fn parses_notify_on_refresh() {
    let raw = r#"
email = "a@b.com"
token = "secret"
notify_on_refresh = true

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.notify_on_refresh);
}

#[test]
fn parses_detail_editable_fields() {
    let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[detail.editable_fields]]
id = "customfield_10042"
label = "Team"
type = "select"
options = ["Alpha", "Beta"]
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert_eq!(cfg.detail.editable_fields.len(), 1);
    assert_eq!(cfg.detail.editable_fields[0].id, "customfield_10042");
    assert_eq!(cfg.detail.editable_fields[0].options.len(), 2);
    let ids = cfg.custom_field_ids_for_fetch();
    assert!(ids.contains(&"customfield_10042".to_string()));
}

#[test]
fn parses_on_config_reload_and_mark_hooks() {
    let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[hooks.on_config_reload]]
command = "true"

[[hooks.on_mark]]
command = "true"
timeout_secs = 5
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert_eq!(cfg.hooks.on_config_reload.len(), 1);
    assert_eq!(cfg.hooks.on_mark.len(), 1);
    assert_eq!(cfg.hooks.on_mark[0].timeout_secs, 5);
}

#[test]
fn parses_on_bulk_complete_hooks() {
    let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[hooks.on_bulk_complete]]
command = "true"
timeout_secs = 10
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert_eq!(cfg.hooks.on_bulk_complete.len(), 1);
    assert_eq!(cfg.hooks.on_bulk_complete[0].command, "true");
    assert_eq!(cfg.hooks.on_bulk_complete[0].timeout_secs, 10);
}

#[test]
fn parses_page_size() {
    let raw = r#"
email = "a@b.com"
token = "secret"
page_size = 25

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert_eq!(cfg.page_size, 25);
    assert!(cfg.validate().is_ok());
}

#[test]
fn rejects_invalid_page_size() {
    let raw = r#"
email = "a@b.com"
token = "secret"
page_size = 0

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
    let mut cfg: Config = toml::from_str(raw).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.validate().is_err());
}

#[test]
fn cli_overrides_validate() {
    let mut cfg: Config = toml::from_str(&sample_config_toml("secret")).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.apply_cli_overrides(Some(100), Some(15)).is_ok());
    assert_eq!(cfg.max_results, 100);
    assert_eq!(cfg.page_size, 15);
    assert!(cfg.apply_cli_overrides(None, Some(0)).is_err());
}

#[test]
fn token_from_env() {
    std::env::set_var("TICK_TOKEN", "from-env");
    let t = Config::resolve_token("").unwrap();
    assert_eq!(t, "from-env");
    std::env::remove_var("TICK_TOKEN");
}

#[test]
fn parses_issue_templates() {
    let toml = r#"
email = "a@b.com"
token = "t"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[create.templates]]
name = "bug"
project = "ONE"
issue_type = "Bug"
summary = "Bug: "
labels = ["a"]
"#;
    let mut cfg: Config = toml::from_str(toml).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.create.templates.len(), 1);
    assert_eq!(cfg.issue_templates_for_site(Some("one")).len(), 1);
}

#[test]
fn multi_site_template_requires_site_field() {
    let toml = r#"
email = "a@b.com"
token = "t"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[sites]]
name = "two"
base_url = "https://two.atlassian.net"

[[create.templates]]
name = "x"
project = "P"
issue_type = "Task"
summary = "S"
"#;
    let mut cfg: Config = toml::from_str(toml).unwrap();
    cfg.view_jql = Config::build_view_jql(&cfg.views);
    assert!(cfg.validate().is_err());
}
