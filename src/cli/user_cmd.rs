use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::config::Config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommand,
}

#[derive(Subcommand, Debug)]
pub enum UserCommand {
    /// Show details of the currently authenticated user
    #[command(name = "me")]
    Me {
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Search for users by display name or email
    #[command(name = "search")]
    Search {
        query: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List users assignable to a project
    #[command(name = "assignable")]
    Assignable {
        project: String,
        #[arg(short = 'q', long, default_value = "")]
        query: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cmd: UserCommand) -> Result<(), String> {
    match cmd {
        UserCommand::Me { site, json } => run_me(site.as_deref(), json).await,
        UserCommand::Search { query, site, json } => {
            run_search(&query, site.as_deref(), json).await
        }
        UserCommand::Assignable {
            project,
            query,
            site,
            json,
        } => run_assignable(&project, &query, site.as_deref(), json).await,
    }
}

async fn run_me(site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let profile = client.get_user_profile(&base_url).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&profile).unwrap_or_default()
        );
        return Ok(());
    }

    let display_name = profile
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let account_id = profile
        .get("accountId")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let email = profile
        .get("emailAddress")
        .and_then(|v| v.as_str())
        .unwrap_or("Hidden");
    let active = profile
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let tz = profile
        .get("timeZone")
        .and_then(|v| v.as_str())
        .unwrap_or("UTC");

    println!(
        "{}  {}",
        style::bold(display_name),
        style::dim(&format!("({email})"))
    );
    println!("{} {}", style::dim("Account ID:"), account_id);
    println!(
        "{} {}",
        style::dim("Active:"),
        if active {
            style::green("yes")
        } else {
            style::red("no")
        }
    );
    println!("{} {}", style::dim("Timezone:"), tz);
    println!("{} {}", style::dim("Site:"), site.name);

    Ok(())
}

async fn run_search(query: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let users = client.search_users(&base_url, query, Some(25)).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&users).unwrap_or_default()
        );
        return Ok(());
    }

    if users.is_empty() {
        println!("{}", style::dim("No users found matching query."));
        return Ok(());
    }

    let headers = ["ACCOUNT ID", "NAME", "EMAIL", "ACTIVE"];
    let rows: Vec<Vec<String>> = users
        .into_iter()
        .map(|u| {
            let id = u
                .get("accountId")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let name = u
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let email = u
                .get("emailAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let active = if u.get("active").and_then(|v| v.as_bool()).unwrap_or(true) {
                "yes".to_string()
            } else {
                "no".to_string()
            };
            vec![id, truncate(&name, 30), truncate(&email, 30), active]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn run_assignable(
    project: &str,
    query: &str,
    site_opt: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let users = client
        .search_assignable_users(&base_url, project, query)
        .await?;

    if json {
        let serialized = serde_json::json!(users
            .iter()
            .map(|(id, name)| {
                serde_json::json!({
                    "accountId": id,
                    "displayName": name
                })
            })
            .collect::<Vec<_>>());
        println!(
            "{}",
            serde_json::to_string_pretty(&serialized).unwrap_or_default()
        );
        return Ok(());
    }

    if users.is_empty() {
        println!("{}", style::dim("No assignable users found."));
        return Ok(());
    }

    let headers = ["ACCOUNT ID", "DISPLAY NAME"];
    let rows: Vec<Vec<String>> = users.into_iter().map(|(id, name)| vec![id, name]).collect();

    format::print_table(&headers, &rows);
    Ok(())
}

fn resolve_site<'a>(
    config: &'a Config,
    site_opt: Option<&str>,
) -> Result<&'a crate::config::Site, String> {
    crate::cli::util::resolve_site_for_command(config, site_opt)
}
