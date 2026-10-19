use crate::api::JiraClient;
use crate::cli::adf_text::render_adf_text;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::cli::util::{self, parse_issue_key_arg, resolve_site};
use crate::config::Config;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct CommentArgs {
    #[command(subcommand)]
    pub action: Option<CommentSubcommand>,
    /// Issue key if adding a comment directly without subcommand
    #[arg(required = false)]
    pub key: Option<String>,
    /// Comment body text (markdown)
    #[arg(short = 'b', long)]
    pub body: Option<String>,
    /// Attach files (repeatable)
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<std::path::PathBuf>,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum CommentSubcommand {
    /// Add a comment to an issue
    Add {
        key: String,
        #[arg(short = 'b', long)]
        body: Option<String>,
        #[arg(long = "attach", value_name = "PATH")]
        attach: Vec<std::path::PathBuf>,
        #[arg(long)]
        site: Option<String>,
    },
    /// List comments on an issue
    List {
        key: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete a comment by ID
    Delete {
        key: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        site: Option<String>,
    },
}

#[derive(Parser, Debug)]
pub struct WorklogArgs {
    #[command(subcommand)]
    pub action: WorklogSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum WorklogSubcommand {
    /// Log work spent on an issue (e.g. "2h 30m")
    Add {
        key: String,
        #[arg(short = 't', long = "time")]
        time_spent: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// List worklogs on an issue
    List {
        key: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete a worklog entry by ID
    Delete {
        key: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        site: Option<String>,
    },
}

#[derive(Parser, Debug)]
pub struct LinkArgs {
    #[command(subcommand)]
    pub action: LinkSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum LinkSubcommand {
    /// Link two issues together
    Add {
        inward_key: String,
        outward_key: String,
        /// Link type name (e.g. "Relates", "Blocks", "Clones", "Duplicate")
        #[arg(short = 't', long, default_value = "Relates")]
        link_type: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// List links for an issue
    List {
        key: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete an issue link by link ID
    Delete {
        #[arg(long)]
        id: String,
        #[arg(long)]
        site: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    pub key: String,
    #[arg(long)]
    pub site: Option<String>,
}

pub async fn run_comment(args: CommentArgs) -> Result<(), String> {
    match args.action {
        Some(CommentSubcommand::List { key, site }) => {
            run_comment_list(&key, site.as_deref()).await
        }
        Some(CommentSubcommand::Delete { key, id, site }) => {
            run_comment_delete(&key, &id, site.as_deref()).await
        }
        Some(CommentSubcommand::Add {
            key,
            body,
            attach,
            site,
        }) => run_comment_add(&key, body, attach, site.as_deref()).await,
        None => {
            let key = args
                .key
                .ok_or_else(|| "Issue key required. See 'tick issue comment --help'".to_string())?;
            run_comment_add(&key, args.body, args.attach, args.site.as_deref()).await
        }
    }
}

async fn run_comment_add(
    key_raw: &str,
    body_opt: Option<String>,
    attach: Vec<std::path::PathBuf>,
    site_opt: Option<&str>,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(key_raw)?;
    let site = resolve_site(&config, &key, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let body = util::read_body_arg(body_opt)?;
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    for p in &attach {
        if !p.is_file() {
            return Err(format!("File does not exist: {}", p.display()));
        }
    }

    client
        .add_comment_with_attachments(&base_url, &key, &body, &[], &attach)
        .await?;
    println!(
        "{} Added comment to {}",
        style::green("✓"),
        style::bold(&key)
    );
    Ok(())
}

async fn run_comment_list(key_raw: &str, site_opt: Option<&str>) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(key_raw)?;
    let site = resolve_site(&config, &key, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let comments = client.list_comments(&base_url, &key).await?;
    if comments.is_empty() {
        println!("{}", style::dim("No comments found."));
        return Ok(());
    }

    for c in &comments {
        let id = c.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let author = c
            .get("author")
            .and_then(|a| a.get("displayName"))
            .and_then(|d| d.as_str())
            .unwrap_or("Unknown");
        let created = c.get("created").and_then(|v| v.as_str()).unwrap_or("");
        let body = c.get("body").map(render_adf_text).unwrap_or_default();

        println!(
            "{} {} ({})",
            style::bold(author),
            style::dim(&format!("[id: {id}]")),
            style::dim(created)
        );
        for line in body.lines() {
            println!("  {line}");
        }
        println!();
    }
    Ok(())
}

async fn run_comment_delete(
    key_raw: &str,
    comment_id: &str,
    site_opt: Option<&str>,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(key_raw)?;
    let site = resolve_site(&config, &key, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    client.delete_comment(&base_url, &key, comment_id).await?;
    println!(
        "{} Deleted comment {comment_id} from {}",
        style::green("✓"),
        style::bold(&key)
    );
    Ok(())
}

pub async fn run_worklog(args: WorklogArgs) -> Result<(), String> {
    match args.action {
        WorklogSubcommand::Add {
            key,
            time_spent,
            site,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = parse_issue_key_arg(&key)?;
            let site = resolve_site(&config, &key, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            client.add_worklog(&base_url, &key, &time_spent).await?;
            println!(
                "{} Logged {} on {}",
                style::green("✓"),
                style::cyan(&time_spent),
                style::bold(&key)
            );
            Ok(())
        }
        WorklogSubcommand::List { key, site } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = parse_issue_key_arg(&key)?;
            let site = resolve_site(&config, &key, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let worklogs = client.list_worklogs(&base_url, &key).await?;
            if worklogs.is_empty() {
                println!("{}", style::dim("No worklogs found."));
                return Ok(());
            }

            let headers = ["ID", "AUTHOR", "TIME SPENT", "STARTED"];
            let rows: Vec<Vec<String>> = worklogs
                .into_iter()
                .map(|w| {
                    let id = w
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let author = w
                        .get("author")
                        .and_then(|a| a.get("displayName"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let time = w
                        .get("timeSpent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let started = w
                        .get("started")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    vec![id, author, time, started]
                })
                .collect();
            format::print_table(&headers, &rows);
            Ok(())
        }
        WorklogSubcommand::Delete { key, id, site } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = parse_issue_key_arg(&key)?;
            let site = resolve_site(&config, &key, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            client.delete_worklog(&base_url, &key, &id).await?;
            println!(
                "{} Deleted worklog {id} from {}",
                style::green("✓"),
                style::bold(&key)
            );
            Ok(())
        }
    }
}

pub async fn run_link(args: LinkArgs) -> Result<(), String> {
    match args.action {
        LinkSubcommand::Add {
            inward_key,
            outward_key,
            link_type,
            site,
        } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = parse_issue_key_arg(&inward_key)?;
            let site = resolve_site(&config, &key, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            client
                .link_issues(&base_url, &link_type, &inward_key, &outward_key)
                .await?;
            println!(
                "{} Linked {} {} {}",
                style::green("✓"),
                style::bold(&inward_key),
                style::cyan(&link_type),
                style::bold(&outward_key)
            );
            Ok(())
        }
        LinkSubcommand::List { key, site } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let key = parse_issue_key_arg(&key)?;
            let site = resolve_site(&config, &key, site.as_deref())?;
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            let relations = client.fetch_issue_relations(&base_url, &key).await?;
            if relations.links.is_empty() {
                println!("{}", style::dim("No linked issues found."));
                return Ok(());
            }

            let headers = ["RELATION", "ISSUE", "SUMMARY", "STATUS"];
            let rows: Vec<Vec<String>> = relations
                .links
                .into_iter()
                .map(|l| {
                    vec![
                        format!("{} ({})", l.link_type, l.direction),
                        l.other_key,
                        truncate(&l.other_summary, 40),
                        l.other_status,
                    ]
                })
                .collect();
            format::print_table(&headers, &rows);
            Ok(())
        }
        LinkSubcommand::Delete { id, site: site_opt } => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let site = if let Some(ref name) = site_opt {
                config
                    .sites
                    .iter()
                    .find(|s| s.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("Unknown site '{name}'"))?
            } else {
                config
                    .sites
                    .first()
                    .ok_or_else(|| "No sites configured".to_string())?
            };
            let base_url = site.base_url.trim_end_matches('/').to_string();
            let client = JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;

            client.delete_issue_link(&base_url, &id).await?;
            println!("{} Deleted issue link {id}", style::green("✓"));
            Ok(())
        }
    }
}

pub async fn run_watch(args: WatchArgs, unwatch: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    if unwatch {
        client.unwatch_issue(&base_url, &key).await?;
        println!(
            "{} Stopped watching {}",
            style::green("✓"),
            style::bold(&key)
        );
    } else {
        client.watch_issue(&base_url, &key).await?;
        println!("{} Now watching {}", style::green("✓"), style::bold(&key));
    }
    Ok(())
}
