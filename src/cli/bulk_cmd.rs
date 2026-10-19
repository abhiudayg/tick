use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::api::JiraClient;
use crate::batch::{self, BatchOutcome};
use crate::cli::util::{parse_keys_list, require_site};
use crate::config::Config;
use crate::hooks;
use crate::operations;

#[derive(Subcommand, Debug)]
pub enum BulkCommand {
    /// Apply workflow transition by name to each key
    Transition(BulkTransitionArgs),
    /// Assign current user to each key
    Assign(BulkAssignArgs),
    /// Replace labels on each key
    Labels(BulkLabelsArgs),
}

#[derive(Parser, Debug)]
pub struct BulkTransitionArgs {
    #[arg(long)]
    pub site: String,
    #[arg(long, value_delimiter = ',')]
    pub keys: Vec<String>,
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct BulkAssignArgs {
    #[arg(long)]
    pub site: String,
    #[arg(long, value_delimiter = ',')]
    pub keys: Vec<String>,
    /// Assign to current Jira user (default)
    #[arg(long, default_value_t = true)]
    pub me: bool,
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct BulkLabelsArgs {
    #[arg(long)]
    pub site: String,
    #[arg(long, value_delimiter = ',')]
    pub keys: Vec<String>,
    #[arg(long)]
    pub set: String,
    #[arg(long)]
    pub quiet: bool,
}

pub async fn run(action: BulkCommand) -> Result<(), String> {
    let result = match action {
        BulkCommand::Transition(a) => run_transition(a).await,
        BulkCommand::Assign(a) => run_assign(a).await,
        BulkCommand::Labels(a) => run_labels(a).await,
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}

async fn run_transition(args: BulkTransitionArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = require_site(&config, &args.site)?;
    let keys = parse_keys_list(&args.keys)?;
    let jira = Arc::new(
        JiraClient::from_config(&config, false)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    let base = site.base_url.trim_end_matches('/').to_string();
    let name = args.to.clone();
    if !args.quiet {
        eprintln!("Bulk transition ({} issues)…", keys.len());
    }
    let outcome =
        batch::run_batch(keys, |key| {
            let jira = jira.clone();
            let base = base.clone();
            let name = name.clone();
            async move {
                operations::transition::apply_transition_by_name(&jira, &base, &key, &name).await
            }
        })
        .await;
    finish_bulk(&config, "Bulk transition", &outcome, args.quiet);
    exit_if_failures(&outcome)
}

async fn run_assign(args: BulkAssignArgs) -> Result<(), String> {
    if !args.me {
        return Err("Only --me assign is supported".into());
    }
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = require_site(&config, &args.site)?;
    let keys = parse_keys_list(&args.keys)?;
    let jira = Arc::new(
        JiraClient::from_config(&config, false)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    let base = site.base_url.trim_end_matches('/').to_string();
    if !args.quiet {
        eprintln!("Bulk assign ({} issues)…", keys.len());
    }
    let account_id = jira.current_user_account_id(&base).await?;
    let aid = account_id.clone();
    let outcome = batch::run_batch(keys, |key| {
        let jira = jira.clone();
        let base = base.clone();
        let aid = aid.clone();
        async move { jira.assign_to_account(&base, &key, &aid).await }
    })
    .await;
    finish_bulk(&config, "Bulk assign", &outcome, args.quiet);
    exit_if_failures(&outcome)
}

async fn run_labels(args: BulkLabelsArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = require_site(&config, &args.site)?;
    let keys = parse_keys_list(&args.keys)?;
    let labels = crate::cli::util::parse_labels_input(&args.set);
    let jira = Arc::new(
        JiraClient::from_config(&config, false)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    let base = site.base_url.trim_end_matches('/').to_string();
    if !args.quiet {
        eprintln!("Bulk labels ({} issues)…", keys.len());
    }
    let outcome = batch::run_batch(keys, |key| {
        let jira = jira.clone();
        let base = base.clone();
        let labels = labels.clone();
        async move { jira.update_labels(&base, &key, &labels).await }
    })
    .await;
    finish_bulk(&config, "Bulk labels", &outcome, args.quiet);
    exit_if_failures(&outcome)
}

fn finish_bulk(config: &Config, label: &str, outcome: &BatchOutcome, quiet: bool) {
    print_bulk_result(label, outcome, quiet);
    hooks::fire_on_bulk_complete(config, label, outcome);
}

fn print_bulk_result(label: &str, outcome: &BatchOutcome, quiet: bool) {
    let json = batch::bulk_result_payload(label, outcome);
    if quiet {
        println!("{}", serde_json::to_string(&json).unwrap_or_default());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    }
    if !quiet && !outcome.failures.is_empty() {
        eprintln!("{}", batch::format_batch_notice(label, outcome));
    }
}

fn exit_if_failures(outcome: &BatchOutcome) -> Result<(), String> {
    if outcome.failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{} issue(s) failed", outcome.failures.len()))
    }
}
