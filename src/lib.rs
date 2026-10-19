pub mod api;
pub mod auth;
pub mod auth_status;
pub mod batch;
pub mod cli;
pub mod columns;
pub mod config;
pub mod config_check;
pub mod hooks;
pub mod issue_key;
pub mod mcp;
pub mod oauth;
pub mod operations;
pub mod platform;
pub mod template_export;
pub mod template_persist;

pub use config::view_mode;
pub use config::ViewMode;

use clap::{Parser, Subcommand};
use cli::style;
use config::Config;

const HELP_TEMPLATE: &str = "\
{before-help}Work seamlessly with Atlassian (Jira and Confluence) from the command line.

USAGE
  tick <command> <subcommand> [flags]

CORE COMMANDS
  auth:        Authenticate tick with Atlassian (OAuth or API token)
  config:      Manage configuration for tick
  context:     Manage and switch active server / site context
  mcp:         Serve Model Context Protocol (MCP) stdio server for LLMs

JIRA COMMANDS
  issue:       Manage Jira issues (list, view, create, edit, close, transition, assign, delete)
  project:     Manage Jira projects and view workflow statuses
  board:       Manage agile boards and view backlogs
  sprint:      Manage agile sprints, sprint issues, and move issues
  field:       Discover Jira standard and custom fields
  user:        View authenticated user, search users, and find assignable users

CONFLUENCE COMMANDS
  space:       View and list Confluence spaces
  page:        Manage Confluence pages (list, view, create, edit, delete, comment, history)

ADDITIONAL COMMANDS
  api:         Make an authenticated Atlassian REST API request (Jira & Confluence)
  bulk:        Perform bulk operations across multiple Jira issues
  doctor:      Probe configured Jira and Confluence endpoints for health and connectivity
  search:      Search Jira issues or Confluence pages
  template:    Export Jira issues as templates

FLAGS
  -h, --help      Show help for command
  -v, --version   Show tick version
      --init      Create default configuration file
      --check     Validate local configuration

LEARN MORE
  Use 'tick <command> <subcommand> --help' for more information about a command.
  Documentation: https://github.com/aeswibon/tick
";

#[derive(Parser, Debug)]
#[command(
    name = "tick",
    version = "2.0.0",
    about = "Fast, comprehensive CLI and MCP server for Atlassian (Jira and Confluence)",
    help_template = HELP_TEMPLATE
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<TickCommand>,

    /// Initialize default config file
    #[arg(long, global = true)]
    pub init: bool,

    /// Offline config validation
    #[arg(long, global = true)]
    pub check: bool,

    /// Run diagnostic checks against configured Atlassian sites
    #[arg(long, global = true)]
    pub doctor: bool,
}

#[derive(Subcommand, Debug)]
pub enum TickCommand {
    /// Authenticate tick with Atlassian (login, status, logout)
    Auth {
        #[command(subcommand)]
        action: AuthCommand,
    },
    /// Manage configuration for tick
    Config(cli::config_cmd::ConfigArgs),
    /// Manage and switch active server/site context
    Context(cli::context_cmd::ContextArgs),
    /// Serve Model Context Protocol (MCP) server for LLMs
    Mcp {
        #[command(subcommand)]
        action: mcp::McpCommand,
    },
    /// Manage Jira issues
    Issue(cli::issue::IssueArgs),
    /// Manage Jira projects
    Project(cli::project_cmd::ProjectArgs),
    /// View agile boards
    Board(cli::agile_cmd::BoardArgs),
    /// View agile sprints
    Sprint(cli::agile_cmd::SprintArgs),
    /// Discover Jira fields
    Fields {
        #[command(subcommand)]
        action: cli::fields_cmd::FieldsCommand,
    },
    /// Jira user directory
    User(cli::user_cmd::UserArgs),
    /// View Confluence spaces
    Space(cli::space_cmd::SpaceArgs),
    /// Manage Confluence pages
    Page(cli::page_cmd::PageArgs),
    /// Raw Atlassian REST API requests
    Api(cli::api_cmd::ApiArgs),
    /// Search Jira issues or Confluence pages
    Search(cli::search::SearchArgs),
    /// Bulk operations across multiple Jira issues
    Bulk {
        #[command(subcommand)]
        action: cli::bulk_cmd::BulkCommand,
    },
    /// Export Jira issues as templates
    Template {
        #[command(subcommand)]
        action: TemplateCommand,
    },
    /// Run diagnostic probes against configured sites
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Browser login; stores tokens in ~/.config/tick/oauth.json
    Login,
    /// Show auth status (API token and/or OAuth session)
    Status,
    /// Remove stored OAuth tokens
    Logout,
}

#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// Fetch issues and emit [[templates]] TOML for config
    Export(TemplateExportArgs),
}

#[derive(Parser, Debug)]
pub struct TemplateExportArgs {
    /// Site name from config ([[sites]].name)
    pub site: String,
    /// Issue keys to export (e.g. PROJ-123)
    pub keys: Vec<String>,
    /// Write to file (append with --append)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    /// Append to an existing templates file
    #[arg(long)]
    pub append: bool,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.init {
        Config::create_default_config().map_err(|e| format!("Config error: {}", e))?;
        println!("{} Initialized default configuration", style::green("✓"));
        return Ok(());
    }

    if cli.check {
        let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
        std::process::exit(config_check::run_check(&config));
    }

    if cli.doctor {
        let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
        run_doctor(&config).await;
        return Ok(());
    }

    let Some(cmd) = cli.command else {
        // No command provided: show gh-formatted help
        print!("{HELP_TEMPLATE}");
        return Ok(());
    };

    match cmd {
        TickCommand::Auth { action } => run_auth(action).await?,
        TickCommand::Config(args) => {
            cli::config_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Context(args) => {
            cli::context_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Mcp { action } => {
            mcp::run(action).await?;
        }
        TickCommand::Issue(args) => {
            cli::issue::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Project(args) => {
            cli::project_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Board(args) => {
            cli::agile_cmd::run_board(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Sprint(args) => {
            cli::agile_cmd::run_sprint(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Fields { action } => {
            cli::fields_cmd::run(action)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::User(args) => {
            cli::user_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Space(args) => {
            cli::space_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Page(args) => {
            cli::page_cmd::run(args.command)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Api(args) => {
            cli::api_cmd::run(args)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Search(args) => {
            cli::search::run(args)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Bulk { action } => {
            cli::bulk_cmd::run(action)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        TickCommand::Template { action } => run_template(action).await?,
        TickCommand::Doctor => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            run_doctor(&config).await;
        }
    }

    Ok(())
}

async fn run_auth(action: AuthCommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        AuthCommand::Login => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            oauth::login(&config.oauth).await?;
        }
        AuthCommand::Status => {
            auth_status::print_status().await?;
        }
        AuthCommand::Logout => {
            oauth::delete_tokens()?;
            println!("{} OAuth tokens removed", style::green("✓"));
        }
    }
    Ok(())
}

async fn run_template(action: TemplateCommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        TemplateCommand::Export(args) => {
            if args.keys.is_empty() {
                return Err(
                    "Provide at least one issue key (e.g. tick template export my-site PROJ-1)"
                        .into(),
                );
            }
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            let jira = api::JiraClient::from_config(&config, false)
                .await
                .map_err(|e| format!("Auth error: {e}"))?;
            let content =
                template_export::export_issues_to_toml(&config, &args.site, &args.keys, &jira)
                    .await?;
            if let Some(path) = args.output {
                template_export::write_templates_file(&path, &content, args.append)?;
                println!("{} Wrote {}", style::green("✓"), path.display());
            } else {
                print!("{content}");
            }
        }
    }
    Ok(())
}

pub async fn run_doctor(config: &Config) {
    let client = match api::JiraClient::from_config(config, false).await {
        Ok(c) => c,
        Err(e) => {
            println!("{} Auth error: {e}", style::red("✗"));
            return;
        }
    };

    println!("{}", style::bold("Atlassian Connectivity Diagnostic:"));
    for site in &config.sites {
        println!(
            "\n{} {} ({})",
            style::cyan("●"),
            style::bold(&site.name),
            style::dim(&site.base_url)
        );
        let probe_jql = "ORDER BY updated DESC";

        match client
            .search_jql(&site.base_url, probe_jql, 3, &mut None)
            .await
        {
            Ok(ids) => println!(
                "  {} Jira Search API: OK ({} sample issues found)",
                style::green("✓"),
                ids.len()
            ),
            Err(e) => println!("  {} Jira Search API: FAILED — {e}", style::red("✗")),
        }

        match client.list_projects(&site.base_url).await {
            Ok(projs) => println!(
                "  {} Jira Projects API: OK ({} projects visible)",
                style::green("✓"),
                projs.len()
            ),
            Err(e) => println!("  {} Jira Projects API: FAILED — {e}", style::red("✗")),
        }

        match client.list_boards(&site.base_url).await {
            Ok(boards) => println!(
                "  {} Jira Agile Boards: OK ({} boards visible)",
                style::green("✓"),
                boards.len()
            ),
            Err(e) => println!("  {} Jira Agile Boards: FAILED — {e}", style::red("✗")),
        }

        match client.confluence_list_spaces(&site.base_url, Some(3)).await {
            Ok(spaces) => println!(
                "  {} Confluence Spaces API: OK ({} spaces visible)",
                style::green("✓"),
                spaces.len()
            ),
            Err(e) => println!(
                "  {} Confluence Spaces API: FAILED/NOT CONFIGURED — {e}",
                style::dim("○")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_text_matches_gh_style_format() {
        assert!(HELP_TEMPLATE.contains("CORE COMMANDS"));
        assert!(HELP_TEMPLATE.contains("JIRA COMMANDS"));
        assert!(HELP_TEMPLATE.contains("CONFLUENCE COMMANDS"));
        assert!(HELP_TEMPLATE.contains("ADDITIONAL COMMANDS"));
        assert!(HELP_TEMPLATE.contains("FLAGS"));
        assert!(HELP_TEMPLATE.contains("LEARN MORE"));
        assert!(HELP_TEMPLATE.contains("tick <command> <subcommand> [flags]"));
    }

    #[test]
    fn test_cli_parsing_subcommands() {
        // Issue list
        let cli = Cli::try_parse_from([
            "tick",
            "issue",
            "list",
            "--project",
            "DEMO",
            "--limit",
            "10",
        ])
        .unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Issue(_))));

        // Issue view
        let cli = Cli::try_parse_from(["tick", "issue", "view", "PROJ-123"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Issue(_))));

        // Project list
        let cli = Cli::try_parse_from(["tick", "project", "list"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Project(_))));

        // Board list
        let cli = Cli::try_parse_from(["tick", "board", "list"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Board(_))));

        // Sprint list
        let cli = Cli::try_parse_from(["tick", "sprint", "list", "42"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Sprint(_))));

        // Space list
        let cli = Cli::try_parse_from(["tick", "space", "list"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Space(_))));

        // Page view
        let cli = Cli::try_parse_from(["tick", "page", "view", "12345"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Page(_))));

        // User me
        let cli = Cli::try_parse_from(["tick", "user", "me"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::User(_))));

        // Raw API
        let cli = Cli::try_parse_from(["tick", "api", "/rest/api/3/myself", "-X", "GET"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Api(_))));

        // Config path
        let cli = Cli::try_parse_from(["tick", "config", "path"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Config(_))));

        // Context list, use, show, clear
        let cli = Cli::try_parse_from(["tick", "context", "list"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Context(_))));
        let cli = Cli::try_parse_from(["tick", "context", "use", "my-site"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Context(_))));
        let cli = Cli::try_parse_from(["tick", "context", "show"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Context(_))));
        let cli = Cli::try_parse_from(["tick", "context", "clear"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Context(_))));

        // Issue transition with rules / list / resolution
        let cli =
            Cli::try_parse_from(["tick", "issue", "transition", "PROJ-123", "--list"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Issue(_))));
        let cli = Cli::try_parse_from([
            "tick",
            "issue",
            "transition",
            "PROJ-123",
            "--to",
            "Done",
            "--resolution",
            "Fixed",
            "-F",
            "customfield_10010=val",
            "-c",
            "Finished ticket",
        ])
        .unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Issue(_))));

        // Project components & versions
        let cli = Cli::try_parse_from(["tick", "project", "components", "DEMO"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Project(_))));
        let cli = Cli::try_parse_from(["tick", "project", "versions", "DEMO"]).unwrap();
        assert!(matches!(cli.command, Some(TickCommand::Project(_))));
    }
}
