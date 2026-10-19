use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::config::Config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct BoardArgs {
    #[command(subcommand)]
    pub command: BoardCommand,
}

#[derive(Subcommand, Debug)]
pub enum BoardCommand {
    /// List agile boards
    #[command(name = "list", alias = "ls")]
    List {
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List issues on an agile board
    #[command(name = "issues")]
    Issues {
        board_id: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List issues in the backlog of an agile board
    #[command(name = "backlog")]
    Backlog {
        board_id: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Parser, Debug)]
pub struct SprintArgs {
    #[command(subcommand)]
    pub command: SprintCommand,
}

#[derive(Subcommand, Debug)]
pub enum SprintCommand {
    /// List active and future sprints for an agile board
    #[command(name = "list", alias = "ls")]
    List {
        board_id: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List issues in a sprint
    #[command(name = "issues")]
    Issues {
        sprint_id: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Move an issue to a sprint or to backlog
    #[command(name = "move")]
    Move {
        issue_key: String,
        /// Target sprint ID or "backlog"
        #[arg(short = 't', long = "to")]
        target: String,
        #[arg(long)]
        site: Option<String>,
    },
}

pub async fn run_board(cmd: BoardCommand) -> Result<(), String> {
    match cmd {
        BoardCommand::List { site, json } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let boards = client.list_boards(&base_url).await?;

            if json {
                let serialized = serde_json::json!(boards
                    .iter()
                    .map(|b| {
                        serde_json::json!({
                            "id": b.id,
                            "name": b.name,
                            "projectKey": b.project_key
                        })
                    })
                    .collect::<Vec<_>>());
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serialized).unwrap_or_default()
                );
                return Ok(());
            }

            if boards.is_empty() {
                println!("{}", style::dim("No boards found."));
                return Ok(());
            }

            let headers = ["ID", "NAME", "PROJECT"];
            let rows: Vec<Vec<String>> = boards
                .into_iter()
                .map(|b| {
                    vec![
                        b.id.to_string(),
                        truncate(&b.name, 40),
                        b.project_key.unwrap_or_default(),
                    ]
                })
                .collect();
            format::print_table(&headers, &rows);
            Ok(())
        }
        BoardCommand::Issues {
            board_id,
            site,
            json,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let issues = client.list_board_issues(&base_url, board_id).await?;
            print_issues_table(issues, json)
        }
        BoardCommand::Backlog {
            board_id,
            site,
            json,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let issues = client.list_backlog_issues(&base_url, board_id).await?;
            print_issues_table(issues, json)
        }
    }
}

pub async fn run_sprint(cmd: SprintCommand) -> Result<(), String> {
    match cmd {
        SprintCommand::List {
            board_id,
            site,
            json,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let sprints = client.list_board_sprints(&base_url, board_id).await?;

            if json {
                let serialized = serde_json::json!(sprints
                    .iter()
                    .map(|(id, name, state)| {
                        serde_json::json!({
                            "id": id,
                            "name": name,
                            "state": state
                        })
                    })
                    .collect::<Vec<_>>());
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serialized).unwrap_or_default()
                );
                return Ok(());
            }

            if sprints.is_empty() {
                println!("{}", style::dim("No active or future sprints found."));
                return Ok(());
            }

            let headers = ["ID", "NAME", "STATE"];
            let rows: Vec<Vec<String>> = sprints
                .into_iter()
                .map(|(id, name, state)| vec![id, truncate(&name, 40), state])
                .collect();
            format::print_table(&headers, &rows);
            Ok(())
        }
        SprintCommand::Issues {
            sprint_id,
            site,
            json,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let issues = client.list_sprint_issues(&base_url, sprint_id).await?;
            print_issues_table(issues, json)
        }
        SprintCommand::Move {
            issue_key,
            target,
            site,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = crate::cli::util::parse_issue_key_arg(&issue_key)?;
            let site = resolve_site(&config, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            client
                .move_issue_to_sprint_target(&base_url, &key, &target)
                .await?;
            println!(
                "{} Moved {} to {}",
                style::green("✓"),
                style::bold(&key),
                style::cyan(&target)
            );
            Ok(())
        }
    }
}

fn print_issues_table(issues: Vec<serde_json::Value>, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&issues).unwrap_or_default()
        );
        return Ok(());
    }

    if issues.is_empty() {
        println!("{}", style::dim("No issues found."));
        return Ok(());
    }

    let headers = ["KEY", "STATUS", "SUMMARY", "ASSIGNEE"];
    let rows: Vec<Vec<String>> = issues
        .into_iter()
        .map(|i| {
            let key = i
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let fields = i.get("fields");
            let summary = fields
                .and_then(|f| f.get("summary"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let status = fields
                .and_then(|f| f.get("status"))
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let assignee = fields
                .and_then(|f| f.get("assignee"))
                .and_then(|a| a.get("displayName"))
                .and_then(|d| d.as_str())
                .unwrap_or("Unassigned")
                .to_string();
            vec![
                key,
                style::status_color(&status),
                truncate(&summary, 45),
                assignee,
            ]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

fn resolve_site<'a>(
    config: &'a Config,
    site_opt: Option<&str>,
) -> Result<&'a crate::config::Site, String> {
    crate::cli::util::resolve_site_for_command(config, site_opt)
}
