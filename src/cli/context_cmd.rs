use crate::cli::style;
use crate::cli::util;
use crate::config::Config;
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(
    name = "context",
    about = "Manage and switch active server/site context",
    help_template = "\
{before-help}Manage and switch active server/site context.

When multiple Atlassian sites or servers are configured, the active context determines
which site commands target by default without needing --site on every invocation.

Precedence order:
  1. CLI flag --site <name>
  2. Environment variable TICK_SITE or TICK_CONTEXT
  3. Active context (set via 'tick context use <name>')
  4. config.toml default_site setting
  5. Single configured site (when only one exists)

USAGE
  tick context <command> [flags]

COMMANDS
  list:        List all configured sites and indicate the active context
  show:        Show current active site context
  use:         Switch active context to a configured site (alias: set)
  clear:       Clear active context (alias: unset)

FLAGS
  -h, --help   Show help for command

EXAMPLES
  $ tick context list
  $ tick context use production
  $ tick context show
  $ tick context clear
"
)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub command: ContextCommand,
}

#[derive(Subcommand, Debug)]
pub enum ContextCommand {
    /// List configured sites and show which is active
    #[command(name = "list", alias = "ls")]
    List {
        /// Output list as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show current active site context
    #[command(name = "show", alias = "current")]
    Show {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch active context to a configured site
    #[command(name = "use", alias = "set")]
    Use {
        /// Name of configured site
        name: String,
    },
    /// Clear active context
    #[command(name = "clear", alias = "unset")]
    Clear,
}

#[derive(Serialize)]
struct SiteContextEntry {
    name: String,
    base_url: String,
    active: bool,
    source: Option<String>,
}

pub async fn run(cmd: ContextCommand) -> Result<(), String> {
    match cmd {
        ContextCommand::List { json } => run_list(json),
        ContextCommand::Show { json } => run_show(json),
        ContextCommand::Use { name } => run_use(&name),
        ContextCommand::Clear => run_clear(),
    }
}

fn run_list(json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let active = util::current_context(&config);

    if json {
        let entries: Vec<SiteContextEntry> = config
            .sites
            .iter()
            .map(|s| {
                let is_active = active.as_ref().map(|a| a.name == s.name).unwrap_or(false);
                let source = if is_active {
                    active.as_ref().map(|a| a.source.to_string())
                } else {
                    None
                };
                SiteContextEntry {
                    name: s.name.clone(),
                    base_url: s.base_url.clone(),
                    active: is_active,
                    source,
                }
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        return Ok(());
    }

    if config.sites.is_empty() {
        println!(
            "{}",
            style::dim("No sites configured in config.toml. Run 'tick --init' to set up a site.")
        );
        return Ok(());
    }

    let headers = ["ACTIVE", "NAME", "BASE URL", "SOURCE"];
    let mut rows = Vec::new();

    for site in &config.sites {
        let is_active = active
            .as_ref()
            .map(|a| a.name == site.name)
            .unwrap_or(false);
        let marker = if is_active {
            style::green("*")
        } else {
            " ".to_string()
        };
        let name_str = if is_active {
            style::bold(&site.name)
        } else {
            site.name.clone()
        };
        let source_str = if is_active {
            active.as_ref().map(|a| a.source).unwrap_or("-").to_string()
        } else {
            "-".to_string()
        };

        rows.push(vec![
            marker,
            name_str,
            site.base_url.clone(),
            style::dim(&source_str),
        ]);
    }

    crate::cli::format::print_table(&headers, &rows);
    Ok(())
}

fn run_show(json: bool) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let active = util::current_context(&config);

    if json {
        let val = serde_json::json!({
            "active_context": active
        });
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        return Ok(());
    }

    match active {
        Some(info) => {
            println!("Active context: {}", style::bold(&info.name));
            println!("URL:            {}", style::blue(&info.base_url));
            println!("Source:         {}", style::dim(info.source));
        }
        None => {
            println!("{}", style::yellow("No active context set."));
            println!(
                "Available sites: {}",
                config
                    .sites
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("Run 'tick context use <site>' to set an active context.");
        }
    }
    Ok(())
}

fn run_use(name: &str) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = config
        .sites
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| {
            let names: Vec<&str> = config.sites.iter().map(|s| s.name.as_str()).collect();
            format!(
                "Site '{name}' not found in config.toml. Configured sites: {}",
                names.join(", ")
            )
        })?;

    Config::set_active_context(&site.name)?;
    println!(
        "{} Switched context to {} ({})",
        style::green("✓"),
        style::bold(&site.name),
        style::blue(&site.base_url)
    );
    Ok(())
}

fn run_clear() -> Result<(), String> {
    Config::clear_active_context()?;
    println!("{} Cleared active context", style::green("✓"));
    Ok(())
}
