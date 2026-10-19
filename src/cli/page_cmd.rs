use crate::api::confluence::storage_format_to_plain_text;
use crate::api::JiraClient;
use crate::cli::format::{self, truncate};
use crate::cli::style;
use crate::cli::util;
use crate::config::Config;
use crate::platform;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct PageArgs {
    #[command(subcommand)]
    pub command: PageCommand,
}

#[derive(Subcommand, Debug)]
pub enum PageCommand {
    /// List pages in a space
    #[command(name = "list", alias = "ls")]
    List {
        /// Space ID (numeric) or key
        space_id: String,
        #[arg(short = 'L', long)]
        limit: Option<u32>,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// View page content and metadata
    #[command(name = "view")]
    View {
        page_id: String,
        /// Show raw Confluence storage format (XHTML) instead of converted text
        #[arg(long)]
        raw: bool,
        /// Open the page in the default web browser
        #[arg(short = 'w', long)]
        web: bool,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a new Confluence page
    #[command(name = "create", alias = "new")]
    Create {
        /// Numeric space ID
        #[arg(short, long)]
        space: String,
        /// Page title
        #[arg(short, long)]
        title: String,
        /// Page content body in markdown or plain text. Reads stdin if omitted.
        #[arg(short = 'b', long)]
        body: Option<String>,
        /// Parent page ID
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Edit an existing Confluence page
    #[command(name = "edit")]
    Edit {
        page_id: String,
        /// Updated page title
        #[arg(short, long)]
        title: String,
        /// Updated body content. Reads stdin if omitted.
        #[arg(short = 'b', long)]
        body: Option<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete / archive a Confluence page
    #[command(name = "delete", alias = "rm")]
    Delete {
        page_id: String,
        #[arg(short = 'y', long)]
        yes: bool,
        #[arg(long)]
        site: Option<String>,
    },
    /// Add or list comments on a page
    #[command(name = "comment")]
    Comment {
        page_id: String,
        /// Comment body. If omitted, lists existing comments.
        #[arg(short = 'b', long)]
        body: Option<String>,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// View revision history for a page
    #[command(name = "history")]
    History {
        page_id: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List attachments on a page
    #[command(name = "attachments")]
    Attachments {
        page_id: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cmd: PageCommand) -> Result<(), String> {
    match cmd {
        PageCommand::List {
            space_id,
            limit,
            site,
            json,
        } => run_list(&space_id, limit, site.as_deref(), json).await,
        PageCommand::View {
            page_id,
            raw,
            web,
            site,
            json,
        } => run_view(&page_id, raw, web, site.as_deref(), json).await,
        PageCommand::Create {
            space,
            title,
            body,
            parent,
            site,
        } => run_create(&space, &title, body, parent.as_deref(), site.as_deref()).await,
        PageCommand::Edit {
            page_id,
            title,
            body,
            site,
        } => run_edit(&page_id, &title, body, site.as_deref()).await,
        PageCommand::Delete { page_id, yes, site } => {
            run_delete(&page_id, yes, site.as_deref()).await
        }
        PageCommand::Comment {
            page_id,
            body,
            site,
            json,
        } => run_comment(&page_id, body, site.as_deref(), json).await,
        PageCommand::History {
            page_id,
            site,
            json,
        } => run_history(&page_id, site.as_deref(), json).await,
        PageCommand::Attachments {
            page_id,
            site,
            json,
        } => run_attachments(&page_id, site.as_deref(), json).await,
    }
}

async fn run_list(
    space_id: &str,
    limit: Option<u32>,
    site_opt: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let pages = client
        .confluence_list_pages_in_space(&site.base_url, space_id, limit)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&pages).unwrap_or_default()
        );
        return Ok(());
    }

    if pages.is_empty() {
        println!("{}", style::dim("No pages found in space."));
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

async fn run_view(
    page_id: &str,
    raw: bool,
    web: bool,
    site_opt: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let page = client.confluence_get_page(&site.base_url, page_id).await?;

    if web {
        if let Some(link) = page.links.as_ref().and_then(|l| l.webui.as_deref()) {
            let full_url = format!("{}{}", site.base_url.trim_end_matches('/'), link);
            let _ = platform::open_url(&full_url);
            println!("Opening {full_url} in browser");
            return Ok(());
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&page).unwrap_or_default()
        );
        return Ok(());
    }

    let version_num = page.version.as_ref().map(|v| v.number).unwrap_or(1);

    println!(
        "{}  {}",
        style::bold(&page.title),
        style::dim(&format!("(v{version_num})"))
    );
    println!("{} {}", style::dim("Page ID:"), page.id);
    println!("{} {}", style::dim("Status:"), page.status);
    if let Some(link) = page.links.as_ref().and_then(|l| l.webui.as_deref()) {
        println!(
            "{} {}\n",
            style::dim("URL:"),
            style::blue(&format!("{}{}", site.base_url.trim_end_matches('/'), link))
        );
    } else {
        println!();
    }

    let body_storage = page
        .body
        .as_ref()
        .and_then(|b| b.storage.as_ref())
        .map(|s| s.value.as_str())
        .unwrap_or("");

    if raw {
        println!("{body_storage}");
    } else {
        println!("{}", storage_format_to_plain_text(body_storage));
    }

    Ok(())
}

async fn run_create(
    space_id: &str,
    title: &str,
    body_opt: Option<String>,
    parent_id: Option<&str>,
    site_opt: Option<&str>,
) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let body_text = util::read_body_arg(body_opt)?;
    let storage_html = crate::api::confluence::plain_text_to_storage_format(&body_text);

    let page = client
        .confluence_create_page(&site.base_url, space_id, title, &storage_html, parent_id)
        .await?;

    println!(
        "{} Created page '{}' (ID: {})",
        style::green("✓"),
        style::bold(&page.title),
        style::bold(&page.id)
    );
    Ok(())
}

async fn run_edit(
    page_id: &str,
    title: &str,
    body_opt: Option<String>,
    site_opt: Option<&str>,
) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let existing = client.confluence_get_page(&site.base_url, page_id).await?;
    let current_version = existing.version.as_ref().map(|v| v.number).unwrap_or(1);

    let storage_html = if let Some(b) = body_opt {
        crate::api::confluence::plain_text_to_storage_format(&b)
    } else {
        existing
            .body
            .as_ref()
            .and_then(|b| b.storage.as_ref())
            .map(|s| s.value.clone())
            .unwrap_or_default()
    };

    let page = client
        .confluence_update_page(
            &site.base_url,
            page_id,
            title,
            &storage_html,
            current_version + 1,
        )
        .await?;

    println!(
        "{} Updated page '{}' to version {}",
        style::green("✓"),
        style::bold(&page.title),
        current_version + 1
    );
    Ok(())
}

async fn run_delete(page_id: &str, yes: bool, site_opt: Option<&str>) -> Result<(), String> {
    if !yes {
        return Err(format!(
            "Deleting page {page_id} is irreversible. Run with '--yes' to confirm deletion."
        ));
    }
    let (client, site) = resolve_client_and_site(site_opt).await?;
    client
        .confluence_delete_page(&site.base_url, page_id)
        .await?;
    println!("{} Deleted page {page_id}", style::green("✓"));
    Ok(())
}

async fn run_comment(
    page_id: &str,
    body_opt: Option<String>,
    site_opt: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;

    if let Some(body) = body_opt {
        let storage_html = crate::api::confluence::plain_text_to_storage_format(&body);
        let comment = client
            .confluence_add_comment(&site.base_url, page_id, &storage_html)
            .await?;
        println!(
            "{} Added comment (ID: {}) to page {page_id}",
            style::green("✓"),
            comment.id
        );
        return Ok(());
    }

    let comments = client
        .confluence_list_comments(&site.base_url, page_id)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&comments).unwrap_or_default()
        );
        return Ok(());
    }

    if comments.is_empty() {
        println!("{}", style::dim("No comments on page."));
        return Ok(());
    }

    for c in &comments {
        let label = c.title.as_deref().unwrap_or("Comment");
        let body = c
            .body
            .as_ref()
            .and_then(|b| b.storage.as_ref())
            .map(|s| storage_format_to_plain_text(&s.value))
            .unwrap_or_default();
        println!(
            "{} {}:",
            style::bold(label),
            style::dim(&format!("[id: {}]", c.id))
        );
        for line in body.lines() {
            println!("  {line}");
        }
        println!();
    }

    Ok(())
}

async fn run_history(page_id: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let versions = client
        .confluence_list_page_versions(&site.base_url, page_id)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&versions).unwrap_or_default()
        );
        return Ok(());
    }

    if versions.is_empty() {
        println!("{}", style::dim("No version history found."));
        return Ok(());
    }

    let headers = ["VERSION", "BY", "CREATED", "MESSAGE"];
    let rows: Vec<Vec<String>> = versions
        .into_iter()
        .map(|v| {
            let num = v
                .get("number")
                .and_then(|n| n.as_u64())
                .unwrap_or(0)
                .to_string();
            let author = v
                .get("by")
                .and_then(|b| b.get("displayName"))
                .and_then(|d| d.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let created = v
                .get("createdAt")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let msg = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            vec![num, author, created, msg]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn run_attachments(page_id: &str, site_opt: Option<&str>, json: bool) -> Result<(), String> {
    let (client, site) = resolve_client_and_site(site_opt).await?;
    let attachments = client
        .confluence_list_attachments(&site.base_url, page_id)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&attachments).unwrap_or_default()
        );
        return Ok(());
    }

    if attachments.is_empty() {
        println!("{}", style::dim("No attachments on page."));
        return Ok(());
    }

    let headers = ["ID", "TITLE", "MEDIA TYPE", "SIZE"];
    let rows: Vec<Vec<String>> = attachments
        .into_iter()
        .map(|a| {
            let id = a
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let title = a
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("?")
                .to_string();
            let media = a
                .get("mediaType")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            let size = a
                .get("fileSize")
                .and_then(|s| s.as_u64())
                .map(|b| format!("{b} B"))
                .unwrap_or_default();
            vec![id, truncate(&title, 40), media, size]
        })
        .collect();

    format::print_table(&headers, &rows);
    Ok(())
}

async fn resolve_client_and_site(
    site_opt: Option<&str>,
) -> Result<(JiraClient, crate::config::Site), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = util::resolve_site_for_command(&config, site_opt)?.clone();
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    Ok((client, site))
}
