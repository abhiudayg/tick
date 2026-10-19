use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::config::Config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct SearchArgs {
    #[command(subcommand)]
    pub target: Option<SearchTarget>,

    /// Query or JQL when run directly without subcommand (e.g. `tick search "error"`)
    #[arg(required = false)]
    pub query: Option<String>,

    /// Limit to specific configured site
    #[arg(long)]
    pub site: Option<String>,

    /// Maximum results
    #[arg(short = 'L', long, default_value = "30")]
    pub limit: u32,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum SearchTarget {
    /// Search Jira issues (by JQL or text)
    Issues {
        query: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(short = 'L', long, default_value = "30")]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    /// Search Confluence pages (by text or CQL)
    Pages {
        query: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(short = 'L', long, default_value = "25")]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(args: SearchArgs) -> Result<(), String> {
    match args.target {
        Some(SearchTarget::Issues {
            query,
            site,
            limit,
            json,
        }) => search_issues(&query, site.as_deref(), limit, json).await,
        Some(SearchTarget::Pages {
            query,
            site,
            limit,
            json,
        }) => search_pages(&query, site.as_deref(), limit, json).await,
        None => {
            let q = args.query.ok_or_else(|| {
                "Search query required. Example: tick search issues 'text ~ \"crash\"'".to_string()
            })?;
            search_issues(&q, args.site.as_deref(), args.limit, args.json).await
        }
    }
}

async fn search_issues(
    query: &str,
    site_opt: Option<&str>,
    limit: u32,
    json: bool,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = if let Some(name) = site_opt {
        config
            .sites
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Unknown site '{name}'"))?
    } else {
        config
            .sites
            .first()
            .ok_or_else(|| "No sites configured in tick config".to_string())?
    };

    let base_url = site.base_url.trim_end_matches('/').to_string();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    // If query contains '=', 'ORDER BY', or 'AND', treat as full JQL, else text search
    let jql = if query.contains('=') || query.to_uppercase().contains("ORDER BY") {
        query.to_string()
    } else {
        format!("text ~ \"{query}\" ORDER BY updated DESC")
    };

    let mut progress = None;
    let ids = client
        .search_jql(&base_url, &jql, limit, &mut progress)
        .await?;
    let issues = client.bulk_fetch(&base_url, &ids, None, &[], false).await?;

    if json {
        let tickets: Vec<crate::cli::util::IssueJson> = issues
            .into_iter()
            .map(|i| {
                let ticket = crate::api::types::Ticket::from_bulk_fetch(
                    i,
                    &site.name,
                    &base_url,
                    None,
                    &[],
                    false,
                );
                crate::cli::util::IssueJson::from_ticket(&ticket, &site.name)
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&tickets).unwrap_or_default()
        );
        return Ok(());
    }

    if issues.is_empty() {
        println!("{}", style::dim("No matching issues found."));
        return Ok(());
    }

    let headers = ["KEY", "STATUS", "SUMMARY", "ASSIGNEE"];
    let rows: Vec<Vec<String>> = issues
        .into_iter()
        .map(|i| {
            let key = i.key;
            let status = i.fields.status.name;
            let summary = i.fields.summary;
            let assignee = i
                .fields
                .assignee
                .map(|a| a.display_name)
                .unwrap_or_else(|| "Unassigned".into());
            vec![
                key,
                style::status_color(&status),
                truncate(&summary, 45),
                assignee,
            ]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn search_pages(
    query: &str,
    site_opt: Option<&str>,
    limit: u32,
    json: bool,
) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = if let Some(name) = site_opt {
        config
            .sites
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Unknown site '{name}'"))?
    } else {
        config
            .sites
            .first()
            .ok_or_else(|| "No sites configured in tick config".to_string())?
    };

    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let pages = client
        .confluence_search_pages(&site.base_url, query, Some(limit))
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&pages).unwrap_or_default()
        );
        return Ok(());
    }

    if pages.is_empty() {
        println!("{}", style::dim("No matching Confluence pages found."));
        return Ok(());
    }

    let headers = ["ID", "TITLE", "STATUS"];
    let rows: Vec<Vec<String>> = pages
        .into_iter()
        .map(|p| vec![p.id, truncate(&p.title, 50), p.status])
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}
