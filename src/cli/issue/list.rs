use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::cli::util::{self, IssueJson};
use crate::config::{Config, Site};
use crate::platform;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct IssueListArgs {
    /// JQL query to filter issues
    #[arg(short, long)]
    pub jql: Option<String>,
    /// Filter by assignee accountId or "me"
    #[arg(short, long)]
    pub assignee: Option<String>,
    /// Filter by status name
    #[arg(long)]
    pub status: Option<String>,
    /// Filter by project key
    #[arg(short, long)]
    pub project: Option<String>,
    /// Maximum number of issues to fetch
    #[arg(short = 'L', long, default_value = "30")]
    pub limit: u32,
    /// Open the search in the web browser
    #[arg(short = 'w', long)]
    pub web: bool,
    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
    /// Target site name from config
    #[arg(short, long)]
    pub site: Option<String>,
}

#[derive(Parser, Debug)]
pub struct IssueViewArgs {
    /// Issue key (e.g. PROJ-123)
    pub key: String,
    /// Include recent comments
    #[arg(short, long)]
    pub comments: bool,
    /// Include linked issues
    #[arg(short, long)]
    pub links: bool,
    /// Open issue in the web browser
    #[arg(short = 'w', long)]
    pub web: bool,
    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
    /// Target site name from config
    #[arg(short, long)]
    pub site: Option<String>,
}

pub async fn run_list(args: IssueListArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = resolve_site(&config, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();

    let jql = build_jql(&args);

    if args.web {
        let web_url = format!("{base_url}/issues/?jql={}", urlencoding(&jql));
        let _ = platform::open_url(&web_url);
        println!("Opening {web_url} in browser");
        return Ok(());
    }

    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let mut progress = None;
    let ids = client
        .search_jql(&base_url, &jql, args.limit, &mut progress)
        .await?;
    let issues = client.bulk_fetch(&base_url, &ids, None, &[], false).await?;

    if args.json {
        let tickets: Vec<IssueJson> = issues
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
                IssueJson::from_ticket(&ticket, &site.name)
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&tickets).unwrap_or_default()
        );
        return Ok(());
    }

    if issues.is_empty() {
        println!("{}", style::dim("No issues found matching query."));
        return Ok(());
    }

    let headers = ["KEY", "STATUS", "PRIORITY", "SUMMARY", "ASSIGNEE"];
    let rows: Vec<Vec<String>> = issues
        .into_iter()
        .map(|i| {
            let key = style::bold(&i.key);
            let status = style::status_color(&i.fields.status.name);
            let prio = style::priority_color(
                i.fields
                    .priority
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("-"),
            );
            let summary = truncate(&i.fields.summary, 60);
            let assignee = i
                .fields
                .assignee
                .as_ref()
                .map(|a| a.display_name.as_str())
                .unwrap_or("-")
                .to_string();
            vec![key, status, prio, summary, assignee]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

pub async fn run_view(args: IssueViewArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = util::parse_issue_key_arg(&args.key)?;
    let site = util::resolve_site(&config, &key, args.site.as_deref())?;
    let base_url = site.base_url.trim_end_matches('/').to_string();

    if args.web {
        let web_url = format!("{base_url}/browse/{key}");
        let _ = platform::open_url(&web_url);
        println!("Opening {web_url} in browser");
        return Ok(());
    }

    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let issues = client
        .bulk_fetch(&base_url, std::slice::from_ref(&key), None, &[], true)
        .await?;
    let issue = issues
        .into_iter()
        .next()
        .ok_or_else(|| format!("Issue {key} not found"))?;
    let detail = client.fetch_issue_detail(&base_url, &key).await.ok();

    if args.json {
        let ticket = crate::api::types::Ticket::from_bulk_fetch(
            issue,
            &site.name,
            &base_url,
            None,
            &[],
            false,
        );
        let issue_json = IssueJson::from_ticket(&ticket, &site.name);
        let val = serde_json::json!({
            "issue": issue_json,
            "detail": detail
        });
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        return Ok(());
    }

    let prio = issue
        .fields
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("-");
    let assignee = issue
        .fields
        .assignee
        .as_ref()
        .map(|a| a.display_name.as_str())
        .unwrap_or("-");
    let reporter = issue
        .fields
        .reporter
        .as_ref()
        .map(|r| r.display_name.as_str())
        .unwrap_or("-");
    let issue_type = &issue.fields.issue_type.name;
    let url = format!("{base_url}/browse/{key}");

    println!(
        "{}  {}",
        style::bold(&key),
        style::bold(&issue.fields.summary)
    );
    println!(
        "{} {}  {} {}  {} {}  {} {}  {} {}",
        style::dim("Type:"),
        style::cyan(issue_type),
        style::dim("Status:"),
        style::status_color(&issue.fields.status.name),
        style::dim("Priority:"),
        style::priority_color(prio),
        style::dim("Assignee:"),
        assignee,
        style::dim("Reporter:"),
        reporter,
    );
    println!("{} {}\n", style::dim("URL:"), style::blue(&url));

    if let Some(ref parent) = issue.fields.parent {
        println!(
            "{} {} - {}",
            style::dim("Parent:"),
            style::bold(&parent.key),
            parent.fields.summary
        );
    }

    if let Some(ref labels) = issue.fields.labels {
        if !labels.is_empty() {
            println!("{} {}", style::dim("Labels:"), labels.join(", "));
        }
    }

    if let Some(comps) = issue
        .fields
        .custom
        .get("components")
        .and_then(|c| c.as_array())
    {
        let comp_names: Vec<&str> = comps
            .iter()
            .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
            .collect();
        if !comp_names.is_empty() {
            println!("{} {}", style::dim("Components:"), comp_names.join(", "));
        }
    }

    if let Some(vers) = issue
        .fields
        .custom
        .get("fixVersions")
        .and_then(|v| v.as_array())
    {
        let ver_names: Vec<&str> = vers
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
            .collect();
        if !ver_names.is_empty() {
            println!("{} {}", style::dim("Fix Versions:"), ver_names.join(", "));
        }
    }

    if let Some(subtasks) = issue
        .fields
        .custom
        .get("subtasks")
        .and_then(|s| s.as_array())
    {
        if !subtasks.is_empty() {
            println!(
                "\n{}",
                style::bold(&format!("SUBTASKS ({})", subtasks.len()))
            );
            for st in subtasks {
                let st_key = st.get("key").and_then(|k| k.as_str()).unwrap_or("?");
                let st_summary = st
                    .get("fields")
                    .and_then(|f| f.get("summary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let st_status = st
                    .get("fields")
                    .and_then(|f| f.get("status"))
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                println!(
                    "  • {} {} [{}]",
                    style::bold(st_key),
                    st_summary,
                    style::status_color(st_status)
                );
            }
        }
    }

    if let Some(links) = issue
        .fields
        .custom
        .get("issuelinks")
        .and_then(|l| l.as_array())
    {
        if !links.is_empty() {
            println!(
                "\n{}",
                style::bold(&format!("ISSUE LINKS ({})", links.len()))
            );
            for link in links {
                let link_type = link
                    .get("type")
                    .and_then(|t| t.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("Relates");
                if let Some(out) = link.get("outwardIssue") {
                    let out_key = out.get("key").and_then(|k| k.as_str()).unwrap_or("?");
                    let out_summary = out
                        .get("fields")
                        .and_then(|f| f.get("summary"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    println!(
                        "  • {} {} ({})",
                        style::bold(out_key),
                        out_summary,
                        style::dim(link_type)
                    );
                } else if let Some(inp) = link.get("inwardIssue") {
                    let in_key = inp.get("key").and_then(|k| k.as_str()).unwrap_or("?");
                    let in_summary = inp
                        .get("fields")
                        .and_then(|f| f.get("summary"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    println!(
                        "  • {} {} ({})",
                        style::bold(in_key),
                        in_summary,
                        style::dim(link_type)
                    );
                }
            }
        }
    }

    println!();
    let desc = detail
        .as_ref()
        .and_then(|d| d.description.as_ref())
        .map(crate::cli::adf_text::render_adf_text)
        .unwrap_or_else(|| "No description provided.".to_string());

    println!("{}\n{}\n", style::bold("DESCRIPTION"), desc.trim());

    if args.comments
        || detail
            .as_ref()
            .map(|d| d.comment.as_ref().map(|c| c.comments.len()).unwrap_or(0))
            .unwrap_or(0)
            > 0
    {
        if let Some(comm) = detail.as_ref().and_then(|d| d.comment.as_ref()) {
            if !comm.comments.is_empty() {
                println!(
                    "{}",
                    style::bold(&format!("COMMENTS ({})", comm.comments.len()))
                );
                for c in &comm.comments {
                    let author = c
                        .author
                        .as_ref()
                        .map(|a| a.display_name.as_str())
                        .unwrap_or("Unknown");
                    let body = c
                        .body
                        .as_ref()
                        .map(crate::cli::adf_text::render_adf_text)
                        .unwrap_or_default();
                    println!("  {} {}:", style::cyan(author), style::dim("commented"));
                    for line in body.lines() {
                        println!("    {line}");
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

fn build_jql(args: &IssueListArgs) -> String {
    if let Some(ref j) = args.jql {
        return j.clone();
    }
    let mut parts = Vec::new();
    if let Some(ref proj) = args.project {
        parts.push(format!("project = \"{proj}\""));
    }
    if let Some(ref st) = args.status {
        parts.push(format!("status = \"{st}\""));
    }
    if let Some(ref a) = args.assignee {
        if a.eq_ignore_ascii_case("me") {
            parts.push("assignee = currentUser()".to_string());
        } else {
            parts.push(format!("assignee = \"{a}\""));
        }
    } else if args.project.is_none() && args.status.is_none() {
        parts.push("assignee = currentUser()".to_string());
    }

    if parts.is_empty() {
        "ORDER BY updated DESC".to_string()
    } else {
        format!("{} ORDER BY updated DESC", parts.join(" AND "))
    }
}

fn resolve_site<'a>(config: &'a Config, site_opt: Option<&str>) -> Result<&'a Site, String> {
    util::resolve_site_for_command(config, site_opt)
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('"', "%22")
        .replace('=', "%3D")
        .replace('&', "%26")
}
