use crate::api::JiraClient;
use crate::cli::style;
use crate::cli::util::{self, parse_issue_key_arg, resolve_site};
use crate::config::Config;
use clap::Parser;
use serde_json::json;
use std::collections::HashMap;

#[derive(Parser, Debug)]
pub struct IssueCreateArgs {
    /// Project key (e.g. PROJ)
    #[arg(short, long)]
    pub project: String,
    /// Issue type (e.g. Task, Bug, Story)
    #[arg(short = 't', long, default_value = "Task")]
    pub issue_type: String,
    /// Issue summary / title
    #[arg(short = 's', long)]
    pub summary: String,
    /// Issue description / body (markdown)
    #[arg(short = 'b', long)]
    pub body: Option<String>,
    /// Assignee account ID, username, or "me"
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,
    /// Comma-separated labels
    #[arg(short = 'l', long)]
    pub labels: Option<String>,
    /// Priority name (e.g. High, Medium, Low)
    #[arg(short = 'p', long)]
    pub priority: Option<String>,
    /// Parent issue key (for sub-tasks or epics)
    #[arg(long)]
    pub parent: Option<String>,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueEditArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// New summary
    #[arg(short = 's', long)]
    pub summary: Option<String>,
    /// New description (markdown)
    #[arg(short = 'b', long)]
    pub body: Option<String>,
    /// New priority
    #[arg(short = 'p', long)]
    pub priority: Option<String>,
    /// Assignee ("me", account ID, or "none")
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,
    /// Labels to set (comma-separated, replaces current labels)
    #[arg(short = 'l', long)]
    pub labels: Option<String>,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueTransitionArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// Target status name or transition name (e.g. "Done", "In Progress")
    #[arg(short = 't', long)]
    pub to: Option<String>,
    /// Workflow resolution (e.g. "Done", "Fixed", "Won't Fix")
    #[arg(short = 'r', long)]
    pub resolution: Option<String>,
    /// Transition comment
    #[arg(short = 'c', long)]
    pub comment: Option<String>,
    /// Custom field in KEY=VALUE format (can be specified multiple times)
    #[arg(short = 'F', long = "field")]
    pub field: Vec<String>,
    /// Raw fields JSON object (e.g. '{"customfield_10010": "val"}')
    #[arg(long = "fields")]
    pub fields_json: Option<String>,
    /// List available transitions from the issue's current status and required fields
    #[arg(long)]
    pub list: bool,
    /// Output transition info as JSON
    #[arg(long)]
    pub json: bool,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueCloseArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// Optional resolution name (e.g. "Done", "Fixed")
    #[arg(short = 'r', long)]
    pub resolution: Option<String>,
    /// Optional resolution comment
    #[arg(short = 'c', long)]
    pub comment: Option<String>,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueReopenArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// Optional reopen comment
    #[arg(short = 'c', long)]
    pub comment: Option<String>,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueAssignArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// User account ID, "me", or "none" / "unassign"
    #[arg(short = 't', long)]
    pub to: Option<String>,
    /// Assign to current user
    #[arg(long, conflicts_with = "unassign")]
    pub me: bool,
    /// Clear assignee
    #[arg(long, conflicts_with = "me")]
    pub unassign: bool,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueDeleteArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// Confirm deletion without interactive prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

pub async fn run_create(args: IssueCreateArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, &args.project, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let mut fields = json!({
        "project": { "key": args.project },
        "issuetype": { "name": args.issue_type },
        "summary": args.summary,
    });

    let body_text = util::read_body_arg(args.body).unwrap_or_default();
    if !body_text.trim().is_empty() {
        let adf = crate::api::markdown::to_adf(&body_text, &[]);
        fields["description"] = adf;
    }

    if let Some(labels_str) = args.labels {
        let labels: Vec<String> = util::parse_labels_input(&labels_str);
        if !labels.is_empty() {
            fields["labels"] = json!(labels);
        }
    }

    if let Some(prio) = args.priority {
        if let Some(id) = client.resolve_priority_id(&base_url, &prio).await {
            fields["priority"] = json!({ "id": id });
        } else {
            fields["priority"] = json!({ "name": prio });
        }
    }

    if let Some(assignee_input) = args.assignee {
        let account_id = if assignee_input.eq_ignore_ascii_case("me") {
            client.current_user_account_id(&base_url).await?
        } else {
            assignee_input
        };
        fields["assignee"] = json!({ "accountId": account_id });
    }

    if let Some(parent) = args.parent {
        fields["parent"] = json!({ "key": parent });
    }

    let key = client
        .create_issue(&base_url, &fields)
        .await
        .map_err(|e| e.message)?;
    let browse_url = format!("{base_url}/browse/{key}");

    println!(
        "{} Created issue {} ({})",
        style::green("✓"),
        style::bold(&key),
        style::blue(&browse_url)
    );
    Ok(())
}

pub async fn run_edit(args: IssueEditArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let mut updated = false;

    if let Some(summary) = args.summary {
        client.update_summary(&base_url, &key, &summary).await?;
        println!("{} Updated summary", style::green("✓"));
        updated = true;
    }

    if let Some(body) = args.body {
        client
            .update_description(&base_url, &key, &body, &[])
            .await?;
        println!("{} Updated description", style::green("✓"));
        updated = true;
    }

    if let Some(priority) = args.priority {
        client.update_priority(&base_url, &key, &priority).await?;
        println!("{} Updated priority to {}", style::green("✓"), priority);
        updated = true;
    }

    if let Some(labels_str) = args.labels {
        let labels = util::parse_labels_input(&labels_str);
        client.update_labels(&base_url, &key, &labels).await?;
        println!("{} Updated labels", style::green("✓"));
        updated = true;
    }

    if let Some(assignee) = args.assignee {
        if assignee.eq_ignore_ascii_case("none") || assignee.eq_ignore_ascii_case("unassign") {
            client.unassign(&base_url, &key).await?;
            println!("{} Removed assignee", style::green("✓"));
        } else if assignee.eq_ignore_ascii_case("me") {
            let me_id = client.current_user_account_id(&base_url).await?;
            client.assign_to_account(&base_url, &key, &me_id).await?;
            println!("{} Assigned to me", style::green("✓"));
        } else {
            client.assign_to_account(&base_url, &key, &assignee).await?;
            println!("{} Assigned to {}", style::green("✓"), assignee);
        }
        updated = true;
    }

    if !updated {
        println!(
            "{}",
            style::dim(
                "No changes specified. Use --summary, --body, --priority, --labels, or --assignee."
            )
        );
    } else {
        println!("{} Updated {}", style::green("✓"), style::bold(&key));
    }
    Ok(())
}

pub async fn run_transition(args: IssueTransitionArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    if args.list || args.to.is_none() {
        let transitions =
            crate::operations::transition::list_transitions_with_fields(&client, &base_url, &key)
                .await?;
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&transitions).unwrap_or_default()
            );
            return Ok(());
        }

        println!("Available workflow transitions for {}:", style::bold(&key));
        if transitions.is_empty() {
            println!(
                "{}",
                style::dim("  No transitions available from current status.")
            );
            return Ok(());
        }

        for t in &transitions {
            let to_part = if t.to_status.is_empty() {
                String::new()
            } else {
                format!(" → {}", style::status_color(&t.to_status))
            };
            println!(
                "  • {} (id: {}){}",
                style::bold(&t.name),
                style::dim(&t.id),
                to_part
            );
            if !t.required_fields.is_empty() {
                println!("    {}", style::yellow("Required fields / rules:"));
                for f in &t.required_fields {
                    if !f.options.is_empty() {
                        println!(
                            "      - {} ({}) [options: {}]",
                            style::bold(&f.name),
                            style::dim(&f.id),
                            f.options.join(", ")
                        );
                    } else {
                        println!("      - {} ({})", style::bold(&f.name), style::dim(&f.id));
                    }
                }
            }
        }
        println!(
            "\nRun 'tick issue transition {} --to <status>' to apply.",
            key
        );
        return Ok(());
    }

    let target = args.to.as_deref().unwrap();
    let mut extra_fields: HashMap<String, serde_json::Value> = HashMap::new();

    if let Some(ref json_str) = args.fields_json {
        let parsed: HashMap<String, serde_json::Value> = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON passed to --fields: {e}"))?;
        for (k, v) in parsed {
            extra_fields.insert(k, v);
        }
    }

    for f in &args.field {
        let parts: Vec<&str> = f.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid -F format '{f}': expected key=value"));
        }
        let k = parts[0].trim().to_string();
        let raw_val = parts[1].trim();
        let v = serde_json::from_str(raw_val)
            .unwrap_or_else(|_| serde_json::Value::String(raw_val.to_string()));
        extra_fields.insert(k, v);
    }

    let options = crate::operations::transition::TransitionOptions {
        resolution: args.resolution.as_deref(),
        extra_fields: Some(&extra_fields),
        comment: args.comment.as_deref(),
    };

    crate::operations::transition::apply_transition_by_name_with_options(
        &client, &base_url, &key, target, &options,
    )
    .await
    .map_err(|e| e.to_string())?;

    println!(
        "{} Transitioned {} to {}",
        style::green("✓"),
        style::bold(&key),
        style::cyan(target)
    );
    Ok(())
}

pub async fn run_close(args: IssueCloseArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let close_targets = ["Done", "Closed", "Resolved", "Complete", "Finish"];
    let mut transitioned = false;
    let options = crate::operations::transition::TransitionOptions {
        resolution: args.resolution.as_deref(),
        extra_fields: None,
        comment: args.comment.as_deref(),
    };
    for target in close_targets {
        if crate::operations::transition::apply_transition_by_name_with_options(
            &client, &base_url, &key, target, &options,
        )
        .await
        .is_ok()
        {
            transitioned = true;
            println!(
                "{} Closed {} as {}",
                style::green("✓"),
                style::bold(&key),
                style::status_color(target)
            );
            break;
        }
    }

    if !transitioned {
        return Err(format!(
            "Could not find a valid transition to close issue {key}. Use 'tick issue transition {key} --list' to inspect available transitions."
        ));
    }

    Ok(())
}

pub async fn run_reopen(args: IssueReopenArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let reopen_targets = ["Reopened", "Reopen", "Open", "To Do", "In Progress"];
    let mut transitioned = false;
    for target in reopen_targets {
        if crate::operations::transition::apply_transition_by_name(&client, &base_url, &key, target)
            .await
            .is_ok()
        {
            transitioned = true;
            println!(
                "{} Reopened {} as {}",
                style::green("✓"),
                style::bold(&key),
                style::cyan(target)
            );
            break;
        }
    }

    if !transitioned {
        return Err(format!(
            "Could not find a valid transition to reopen issue {key}. Use 'tick issue transition {key} --to <status>'."
        ));
    }

    if let Some(comment) = args.comment {
        if !comment.trim().is_empty() {
            client
                .add_comment_with_attachments(&base_url, &key, &comment, &[], &[])
                .await?;
            println!("{} Added reopen comment", style::green("✓"));
        }
    }

    Ok(())
}

pub async fn run_assign(args: IssueAssignArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    if args.unassign || args.to.as_deref() == Some("none") || args.to.as_deref() == Some("unassign")
    {
        client.unassign(&base_url, &key).await?;
        println!("{} Unassigned {}", style::green("✓"), style::bold(&key));
    } else if args.me || args.to.as_deref() == Some("me") {
        let me_id = client.current_user_account_id(&base_url).await?;
        client.assign_to_account(&base_url, &key, &me_id).await?;
        println!(
            "{} Assigned {} to {}",
            style::green("✓"),
            style::bold(&key),
            style::cyan("me")
        );
    } else if let Some(target) = args.to {
        client.assign_to_account(&base_url, &key, &target).await?;
        println!(
            "{} Assigned {} to {}",
            style::green("✓"),
            style::bold(&key),
            style::cyan(&target)
        );
    } else {
        return Err("Specify --to <account_id|me|none>, --me, or --unassign".to_string());
    }

    Ok(())
}

pub async fn run_delete(args: IssueDeleteArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_issue_key_arg(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    if !args.yes {
        return Err(format!(
            "Deleting issue {key} is irreversible. Run with '--yes' to confirm deletion."
        ));
    }

    client.delete_issue(&base_url, &key).await?;
    println!("{} Deleted issue {}", style::green("✓"), style::bold(&key));
    Ok(())
}
