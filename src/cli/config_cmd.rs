use crate::cli::style;
use crate::config::Config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Print the value of a given configuration key
    #[command(name = "get")]
    Get { key: String },
    /// Open or display the path to the configuration file
    #[command(name = "path")]
    Path,
    /// List all configured sites and views
    #[command(name = "list", alias = "ls")]
    List,
}

pub async fn run(cmd: ConfigCommand) -> Result<(), String> {
    match cmd {
        ConfigCommand::Get { key } => {
            let path = Config::config_path()?;
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read config at {}: {e}", path.display()))?;
            let val: toml::Value =
                toml::from_str(&content).map_err(|e| format!("Cannot parse TOML config: {e}"))?;

            if let Some(target) = val.get(&key) {
                println!("{target}");
            } else {
                return Err(format!("Key '{key}' not found in configuration"));
            }
            Ok(())
        }
        ConfigCommand::Path => {
            let path = Config::config_path()?;
            println!("{}", path.display());
            Ok(())
        }
        ConfigCommand::List => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            println!("{}", style::bold("Configured Sites:"));
            for s in &config.sites {
                println!("  {} -> {}", style::cyan(&s.name), style::blue(&s.base_url));
            }
            println!(
                "\n{} {}",
                style::dim("Auth method:"),
                match config.auth {
                    crate::config::AuthMethod::Token => "API Token",
                    crate::config::AuthMethod::Oauth => "OAuth 2.0 (3LO)",
                }
            );
            if !config.email.is_empty() {
                println!("{} {}", style::dim("User email:"), config.email);
            }
            Ok(())
        }
    }
}
