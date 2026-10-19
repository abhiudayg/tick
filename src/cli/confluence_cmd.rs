use clap::{Parser, Subcommand};
use std::io::Read;

use crate::api::confluence::storage_format_to_plain_text;
use crate::api::JiraClient;
use crate::config::{Config, Site};

#[derive(Parser)]
pub struct SpacesArgs {
    #[arg(long)]
    pub site: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long)]
    pub site: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct ViewArgs {
    pub page_id: String,
    #[arg(long)]
    pub site: Option<String>,
    #[arg(long)]
    pub raw: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct CreateArgs {
    #[arg(long)]
    pub space: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser)]
pub struct EditArgs {
    pub page_id: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser)]
pub struct DeleteArgs {
    pub page_id: String,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser)]
pub struct CommentsArgs {
    pub page_id: String,
    #[arg(long)]
    pub site: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct CommentAddArgs {
    pub page_id: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Subcommand)]
pub enum ConfluenceCommand {
    /// List Confluence spaces
    Spaces(SpacesArgs),
    /// Search Confluence pages (CQL or query)
    Search(SearchArgs),
    /// View page content
    View(ViewArgs),
    /// Create a new page
    Create(CreateArgs),
    /// Edit an existing page
    Edit(EditArgs),
    /// Delete / archive a page
    Delete(DeleteArgs),
    /// List comments on a page
    Comments(CommentsArgs),
    /// Add a comment to a page
    CommentAdd(CommentAddArgs),
}

pub async fn run(action: ConfluenceCommand) -> Result<(), Box<dyn std::error::Error>> {
    let result = match action {
        ConfluenceCommand::Spaces(args) => run_spaces(args).await,
        ConfluenceCommand::Search(args) => run_search(args).await,
        ConfluenceCommand::View(args) => run_view(args).await,
        ConfluenceCommand::Create(args) => run_create(args).await,
        ConfluenceCommand::Edit(args) => run_edit(args).await,
        ConfluenceCommand::Delete(args) => run_delete(args).await,
        ConfluenceCommand::Comments(args) => run_comments(args).await,
        ConfluenceCommand::CommentAdd(args) => run_comment_add(args).await,
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}

async fn resolve_client_and_site(site_opt: Option<&str>) -> Result<(JiraClient, Site), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = if let Some(name) = site_opt {
        config
            .sites
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| format!("Unknown site '{name}'"))?
    } else {
        config
            .sites
            .first()
            .cloned()
            .ok_or_else(|| "No sites configured in tick config".to_string())?
    };
    let client = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    Ok((client, site))
}

fn read_body(body_arg: Option<String>) -> Result<String, String> {
    if let Some(b) = body_arg {
        let trimmed = b.trim();
        if trimmed.is_empty() {
            return Err("Body cannot be empty".into());
        }
        return Ok(trimmed.to_string());
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("Stdin read error: {e}"))?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Err("Body cannot be empty (stdin was empty)".into());
    }
    Ok(trimmed.to_string())
}

async fn run_spaces(args: SpacesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let spaces = client
        .confluence_list_spaces(&site.base_url, args.limit)
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&spaces)?);
    } else {
        println!("Spaces on {}:", site.name);
        for s in spaces {
            println!("  [{}] {} (id: {})", s.key, s.name, s.id);
        }
    }
    Ok(())
}

async fn run_search(args: SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let pages = client
        .confluence_search_pages(&site.base_url, &args.query, args.limit)
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&pages)?);
    } else {
        println!("Search results for '{}' on {}:", args.query, site.name);
        for p in pages {
            println!("  #{} — {} [{}]", p.id, p.title, p.status);
        }
    }
    Ok(())
}

async fn run_view(args: ViewArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let page = client
        .confluence_get_page(&site.base_url, &args.page_id)
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&page)?);
        return Ok(());
    }

    let version_num = page.version.as_ref().map(|v| v.number).unwrap_or(1);
    println!("=== {} (id: {}, v{}) ===", page.title, page.id, version_num);
    if let Some(links) = &page.links {
        if let Some(webui) = &links.webui {
            println!("URL: {}{}", site.base_url.trim_end_matches('/'), webui);
        }
    }
    println!("--------------------------------------------------");

    let body_storage = page
        .body
        .as_ref()
        .and_then(|b| b.storage.as_ref().or(b.view.as_ref()))
        .map(|c| c.value.as_str())
        .unwrap_or("");

    if args.raw {
        println!("{body_storage}");
    } else {
        let text = storage_format_to_plain_text(body_storage);
        println!("{text}");
    }

    Ok(())
}

async fn run_create(args: CreateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let body = read_body(args.body)?;
    let created = client
        .confluence_create_page(
            &site.base_url,
            &args.space,
            &args.title,
            &body,
            args.parent.as_deref(),
        )
        .await?;
    println!(
        "Created page #{} \"{}\" in space {}",
        created.id, created.title, args.space
    );
    Ok(())
}

async fn run_edit(args: EditArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let existing = client
        .confluence_get_page(&site.base_url, &args.page_id)
        .await?;
    let cur_version = existing.version.map(|v| v.number).unwrap_or(1);
    let body = read_body(args.body)?;
    let updated = client
        .confluence_update_page(
            &site.base_url,
            &args.page_id,
            &args.title,
            &body,
            cur_version + 1,
        )
        .await?;
    println!("Updated page #{} \"{}\"", updated.id, updated.title);
    Ok(())
}

async fn run_delete(args: DeleteArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    client
        .confluence_delete_page(&site.base_url, &args.page_id)
        .await?;
    println!("Deleted page #{}", args.page_id);
    Ok(())
}

async fn run_comments(args: CommentsArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let comments = client
        .confluence_list_comments(&site.base_url, &args.page_id)
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&comments)?);
    } else {
        println!("Comments for page #{}:", args.page_id);
        for c in comments {
            let body = c
                .body
                .as_ref()
                .and_then(|b| b.storage.as_ref().or(b.view.as_ref()))
                .map(|v| v.value.as_str())
                .unwrap_or("");
            let text = storage_format_to_plain_text(body);
            println!("  [Comment #{}] {}", c.id, text.trim());
        }
    }
    Ok(())
}

async fn run_comment_add(args: CommentAddArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (client, site) = resolve_client_and_site(args.site.as_deref()).await?;
    let body = read_body(args.body)?;
    let comment = client
        .confluence_add_comment(&site.base_url, &args.page_id, &body)
        .await?;
    println!("Added comment #{} to page #{}", comment.id, args.page_id);
    Ok(())
}
