use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::config::Config;
use crate::platform;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
    /// List all accessible Jira projects
    #[command(name = "list", alias = "ls")]
    List {
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// View project details
    #[command(name = "view")]
    View {
        key: String,
        #[arg(short = 'w', long)]
        web: bool,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List issue types and workflow statuses configured for a project
    #[command(name = "statuses")]
    Statuses {
        key: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List components defined in a project
    #[command(name = "components")]
    Components {
        key: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List release versions defined in a project
    #[command(name = "versions")]
    Versions {
        key: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cmd: ProjectCommand) -> Result<(), String> {
    match cmd {
        ProjectCommand::List { site, json } => run_list(site.as_deref(), json).await,
        ProjectCommand::View {
            key,
            web,
            site,
            json,
        } => run_view(&key, web, site.as_deref(), json).await,
        ProjectCommand::Statuses { key, site, json } => {
            run_statuses(&key, site.as_deref(), json).await
        }
        ProjectCommand::Components { key, site, json } => {
            run_components(&key, site.as_deref(), json).await
        }
        ProjectCommand::Versions { key, site, json } => {
            run_versions(&key, site.as_deref(), json).await
        }
    }
}

async fn run_list(site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let projects = client.list_projects(&base_url).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&projects).unwrap_or_default()
        );
        return Ok(());
    }

    if projects.is_empty() {
        println!("{}", style::dim("No projects found."));
        return Ok(());
    }

    let headers = ["KEY", "NAME", "PROJECT TYPE", "LEAD"];
    let rows: Vec<Vec<String>> = projects
        .iter()
        .map(|p| {
            let key = p
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let name = p
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let ptype = p
                .get("projectTypeKey")
                .and_then(|v| v.as_str())
                .unwrap_or("standard")
                .to_string();
            let lead = p
                .get("lead")
                .and_then(|l| l.get("displayName"))
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            vec![key, truncate(&name, 35), ptype, lead]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn run_view(key: &str, web: bool, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();

    if web {
        let web_url = format!("{base_url}/jira/projects/{key}");
        let _ = platform::open_url(&web_url);
        println!("Opening {web_url} in browser");
        return Ok(());
    }

    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let p = client.get_project(&base_url, key).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&p).unwrap_or_default());
        return Ok(());
    }

    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or(key);
    let desc = p
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("None");
    let lead = p
        .get("lead")
        .and_then(|l| l.get("displayName"))
        .and_then(|d| d.as_str())
        .unwrap_or("Unknown");
    let category = p
        .get("projectCategory")
        .and_then(|c| c.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("None");
    let ptype = p
        .get("projectTypeKey")
        .and_then(|v| v.as_str())
        .unwrap_or("software");

    println!("{}  {}", style::bold(key), style::bold(name));
    println!("{} {}", style::dim("Category:"), category);
    println!("{} {}", style::dim("Type:"), style::cyan(ptype));
    println!("{} {}", style::dim("Lead:"), lead);
    println!(
        "{} {}\n",
        style::dim("URL:"),
        style::blue(&format!("{base_url}/jira/projects/{key}"))
    );
    println!("{}\n{}", style::bold("DESCRIPTION"), desc);

    Ok(())
}

async fn run_statuses(key: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let issue_types = client.get_project_statuses(&base_url, key).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&issue_types).unwrap_or_default()
        );
        return Ok(());
    }

    if issue_types.is_empty() {
        println!("{}", style::dim("No statuses found for project."));
        return Ok(());
    }

    for it in &issue_types {
        let it_name = it
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Issue Type");
        println!("{}", style::bold(&format!("Issue Type: {it_name}")));
        if let Some(statuses) = it.get("statuses").and_then(|s| s.as_array()) {
            let headers = ["STATUS ID", "STATUS NAME", "CATEGORY"];
            let rows: Vec<Vec<String>> = statuses
                .iter()
                .map(|s| {
                    let id = s
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let name = s
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let cat = s
                        .get("statusCategory")
                        .and_then(|c| c.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    vec![id, name, cat]
                })
                .collect();
            format::print_table(&headers, &rows);
        }
        println!();
    }

    Ok(())
}

async fn run_components(key: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let components = client.get_project_components(&base_url, key).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&components).unwrap_or_default()
        );
        return Ok(());
    }

    if components.is_empty() {
        println!("{}", style::dim("No components found for project."));
        return Ok(());
    }

    let headers = ["ID", "NAME", "LEAD", "DESCRIPTION"];
    let rows: Vec<Vec<String>> = components
        .iter()
        .map(|c| {
            let id = c
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let name = c
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let lead = c
                .get("lead")
                .and_then(|l| l.get("displayName"))
                .and_then(|d| d.as_str())
                .unwrap_or("-")
                .to_string();
            let desc = c
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string();
            vec![id, name, lead, truncate(&desc, 40)]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn run_versions(key: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let versions = client.get_project_versions(&base_url, key).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&versions).unwrap_or_default()
        );
        return Ok(());
    }

    if versions.is_empty() {
        println!("{}", style::dim("No versions found for project."));
        return Ok(());
    }

    let headers = ["ID", "NAME", "RELEASED", "ARCHIVED", "RELEASE DATE"];
    let rows: Vec<Vec<String>> = versions
        .iter()
        .map(|v| {
            let id = v
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
                .to_string();
            let name = v
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
                .to_string();
            let released = if v.get("released").and_then(|x| x.as_bool()).unwrap_or(false) {
                style::green("yes")
            } else {
                style::dim("no")
            };
            let archived = if v.get("archived").and_then(|x| x.as_bool()).unwrap_or(false) {
                style::yellow("yes")
            } else {
                style::dim("no")
            };
            let rel_date = v
                .get("releaseDate")
                .and_then(|x| x.as_str())
                .unwrap_or("-")
                .to_string();
            vec![id, name, released, archived, rel_date]
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
