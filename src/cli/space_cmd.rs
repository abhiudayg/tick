use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::config::Config;
use crate::platform;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct SpaceArgs {
    #[command(subcommand)]
    pub command: SpaceCommand,
}

#[derive(Subcommand, Debug)]
pub enum SpaceCommand {
    /// List Confluence spaces
    #[command(name = "list", alias = "ls")]
    List {
        #[arg(short = 'L', long)]
        limit: Option<u32>,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// View details of a Confluence space
    #[command(name = "view")]
    View {
        space_key_or_id: String,
        #[arg(short = 'w', long)]
        web: bool,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cmd: SpaceCommand) -> Result<(), String> {
    match cmd {
        SpaceCommand::List { limit, site, json } => run_list(limit, site.as_deref(), json).await,
        SpaceCommand::View {
            space_key_or_id,
            web,
            site,
            json,
        } => run_view(&space_key_or_id, web, site.as_deref(), json).await,
    }
}

async fn run_list(limit: Option<u32>, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let spaces = client.confluence_list_spaces(&base_url, limit).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&spaces).unwrap_or_default()
        );
        return Ok(());
    }

    if spaces.is_empty() {
        println!("{}", style::dim("No spaces found."));
        return Ok(());
    }

    let headers = ["ID", "KEY", "NAME", "TYPE", "STATUS"];
    let rows: Vec<Vec<String>> = spaces
        .into_iter()
        .map(|s| vec![s.id, s.key, truncate(&s.name, 35), s.space_type, s.status])
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn run_view(
    space_key_or_id: &str,
    web: bool,
    site_opt: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, site_opt)?;
    let base_url = site.base_url.trim_end_matches('/').to_string();

    if web {
        let web_url = format!("{base_url}/wiki/spaces/{space_key_or_id}");
        let _ = platform::open_url(&web_url);
        println!("Opening {web_url} in browser");
        return Ok(());
    }

    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let space = client
        .confluence_get_space(&base_url, space_key_or_id)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&space).unwrap_or_default()
        );
        return Ok(());
    }

    println!("{}  {}", style::bold(&space.key), style::bold(&space.name));
    println!("{} {}", style::dim("Space ID:"), space.id);
    println!("{} {}", style::dim("Type:"), style::cyan(&space.space_type));
    println!("{} {}", style::dim("Status:"), space.status);
    if let Some(ref hp) = space.homepage_id {
        println!("{} {}", style::dim("Homepage ID:"), hp);
    }
    println!(
        "{} {}\n",
        style::dim("URL:"),
        style::blue(&format!("{base_url}/wiki/spaces/{}", space.key))
    );

    Ok(())
}

fn resolve_site<'a>(
    config: &'a Config,
    site_opt: Option<&str>,
) -> Result<&'a crate::config::Site, String> {
    crate::cli::util::resolve_site_for_command(config, site_opt)
}
