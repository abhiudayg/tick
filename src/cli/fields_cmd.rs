use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::api::fields::FieldCatalogEntry;
use crate::api::JiraClient;
use crate::cli::util::require_site;
use crate::config::Config;

#[derive(Subcommand, Debug)]
pub enum FieldsCommand {
    /// List custom fields with suggested [[detail.editable_fields]] types (JSON)
    List(FieldsListArgs),
}

#[derive(Parser, Debug)]
pub struct FieldsListArgs {
    #[arg(long)]
    pub site: String,
    /// Include standard (non-custom) fields
    #[arg(long)]
    pub all: bool,
    /// Enrich select fields with options from project create screen
    #[arg(long)]
    pub project: Option<String>,
}

#[derive(Serialize)]
struct FieldsListOutput {
    site: String,
    fields: Vec<FieldListRow>,
}

#[derive(Serialize)]
struct FieldListRow {
    #[serde(flatten)]
    entry: FieldCatalogEntry,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_snippet: Option<String>,
}

pub async fn run(action: FieldsCommand) -> Result<(), String> {
    match action {
        FieldsCommand::List(args) => run_list(args).await,
    }
}

async fn run_list(args: FieldsListArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = require_site(&config, &args.site)?;
    let jira = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    let base = site.base_url.trim_end_matches('/');

    let catalog = jira.list_field_catalog(base, !args.all).await?;

    let createmeta = if let Some(ref project) = args.project {
        Some(
            jira.list_createmeta_fields_for_project(base, project)
                .await?,
        )
    } else {
        None
    };

    let rows: Vec<FieldListRow> = catalog
        .into_iter()
        .map(|entry| {
            let options = createmeta
                .as_ref()
                .and_then(|meta| enrich_options(meta, &entry.id));
            let config_snippet = Some(format_config_snippet(&entry, options.as_deref()));
            FieldListRow {
                entry,
                options,
                config_snippet,
            }
        })
        .collect();

    let out = FieldsListOutput {
        site: site.name.clone(),
        fields: rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn enrich_options(createmeta_fields: &serde_json::Value, field_id: &str) -> Option<Vec<String>> {
    let meta = createmeta_fields.get(field_id)?;
    let labels: Vec<String> = meta
        .get("allowedValues")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.get("value")
                        .or_else(|| v.get("name"))
                        .and_then(|x| x.as_str())
                        .map(String::from)
                })
                .collect::<Vec<_>>()
        })
        .filter(|v: &Vec<String>| !v.is_empty())?;
    Some(labels)
}

fn format_config_snippet(entry: &FieldCatalogEntry, options: Option<&[String]>) -> String {
    let label = entry.name.replace('"', "\\\"");
    let ty = if entry.suggested_type == "unsupported"
        || ((entry.suggested_type == "select" || entry.suggested_type == "multiselect")
            && options.is_none())
    {
        "auto"
    } else {
        entry.suggested_type.as_str()
    };
    let mut lines = vec![
        "[[detail.editable_fields]]".to_string(),
        format!("id = \"{}\"", entry.id),
        format!("label = \"{label}\""),
        format!("type = \"{ty}\""),
    ];
    if ty == "select" || ty == "multiselect" {
        if let Some(opts) = options {
            if !opts.is_empty() {
                let joined = opts
                    .iter()
                    .map(|o| format!("\"{}\"", o.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("options = [{joined}]"));
            }
        }
    }
    lines.join("\n")
}
