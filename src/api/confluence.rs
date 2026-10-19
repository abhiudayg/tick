use crate::api::JiraClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluenceSpace {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(rename = "type", default)]
    pub space_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "homepageId")]
    pub homepage_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluencePage {
    pub id: String,
    pub status: String,
    pub title: String,
    #[serde(rename = "spaceId", default)]
    pub space_id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub version: Option<ConfluenceVersion>,
    #[serde(default)]
    pub body: Option<ConfluenceBody>,
    #[serde(rename = "_links", default)]
    pub links: Option<ConfluenceLinks>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfluenceVersion {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfluenceBody {
    #[serde(default)]
    pub storage: Option<ConfluenceBodyContent>,
    #[serde(default)]
    pub view: Option<ConfluenceBodyContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfluenceBodyContent {
    pub value: String,
    #[serde(default)]
    pub representation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfluenceLinks {
    #[serde(default)]
    pub webui: Option<String>,
    #[serde(rename = "tinyui", default)]
    pub tinyui: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluenceComment {
    pub id: String,
    pub status: String,
    pub title: Option<String>,
    pub body: Option<ConfluenceBody>,
    pub version: Option<ConfluenceVersion>,
}

#[derive(Deserialize)]
struct PaginatedResults<T> {
    results: Vec<T>,
}

#[derive(Deserialize)]
struct CqlSearchResults {
    results: Vec<CqlContentItem>,
}

#[derive(Deserialize)]
struct CqlContentItem {
    id: String,
    title: String,
    status: Option<String>,
    #[serde(rename = "_links", default)]
    links: Option<ConfluenceLinks>,
}

impl JiraClient {
    /// List spaces in Confluence instance
    pub async fn confluence_list_spaces(
        &self,
        base_url: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ConfluenceSpace>, String> {
        let lim = limit.unwrap_or(25);
        let url = format!(
            "{}/wiki/api/v2/spaces?limit={}",
            base_url.trim_end_matches('/'),
            lim
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Confluence spaces API {}: {}", status, body));
        }
        let data: PaginatedResults<ConfluenceSpace> = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data.results)
    }

    /// Get details of a single space
    pub async fn confluence_get_space(
        &self,
        base_url: &str,
        space_id_or_key: &str,
    ) -> Result<ConfluenceSpace, String> {
        let base = base_url.trim_end_matches('/');
        // Try v2 by id first, fallback to v2 with keys query
        let url = format!("{}/wiki/api/v2/spaces/{}", base, space_id_or_key);
        let resp = self.send(|| self.get(&url).send()).await?;
        if resp.status().is_success() {
            return resp
                .json::<ConfluenceSpace>()
                .await
                .map_err(|e| format!("Parse error: {}", e));
        }

        // Search by space key
        let url_key = format!("{}/wiki/api/v2/spaces?keys={}", base, space_id_or_key);
        let resp_key = self.send(|| self.get(&url_key).send()).await?;
        if resp_key.status().is_success() {
            let data: PaginatedResults<ConfluenceSpace> = resp_key
                .json()
                .await
                .map_err(|e| format!("Parse error: {}", e))?;
            if let Some(space) = data.results.into_iter().next() {
                return Ok(space);
            }
        }

        Err(format!("Space '{space_id_or_key}' not found"))
    }

    /// List pages in a space
    pub async fn confluence_list_pages_in_space(
        &self,
        base_url: &str,
        space_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ConfluencePage>, String> {
        let lim = limit.unwrap_or(25);
        let url = format!(
            "{}/wiki/api/v2/spaces/{}/pages?limit={}",
            base_url.trim_end_matches('/'),
            space_id,
            lim
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Confluence pages API {}: {}", status, body));
        }
        let data: PaginatedResults<ConfluencePage> = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data.results)
    }

    /// Search pages with CQL (or text title search)
    pub async fn confluence_search_pages(
        &self,
        base_url: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ConfluencePage>, String> {
        let lim = limit.unwrap_or(25);
        let base = base_url.trim_end_matches('/');

        // Construct CQL query: if contains space or keywords, use CQL directly, else title/text match
        let cql = if query.contains('=') || query.contains("type =") {
            query.to_string()
        } else {
            format!(
                "type = page and (title ~ \"{}\" or text ~ \"{}\")",
                query, query
            )
        };

        let lim_str = lim.to_string();
        let url = format!("{}/wiki/rest/api/content/search", base);
        let resp = self
            .send(|| {
                self.get(&url)
                    .query(&[("cql", &cql), ("limit", &lim_str)])
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            let data: CqlSearchResults = resp
                .json()
                .await
                .map_err(|e| format!("Parse error: {}", e))?;
            let pages = data
                .results
                .into_iter()
                .map(|item| ConfluencePage {
                    id: item.id,
                    title: item.title,
                    status: item.status.unwrap_or_else(|| "current".into()),
                    space_id: String::new(),
                    parent_id: None,
                    version: None,
                    body: None,
                    links: item.links,
                })
                .collect();
            return Ok(pages);
        }

        // Fallback to v2 title search
        let url_v2 = format!("{}/wiki/api/v2/pages", base);
        let resp_v2 = self
            .send(|| {
                self.get(&url_v2)
                    .query(&[("title", query), ("limit", &lim_str)])
                    .send()
            })
            .await?;
        if !resp_v2.status().is_success() {
            let status = resp_v2.status();
            let body = resp_v2.text().await.unwrap_or_default();
            return Err(format!("Confluence search error {}: {}", status, body));
        }
        let data_v2: PaginatedResults<ConfluencePage> = resp_v2
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data_v2.results)
    }

    /// Get page by ID with storage/view body formats
    pub async fn confluence_get_page(
        &self,
        base_url: &str,
        page_id: &str,
    ) -> Result<ConfluencePage, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}?body-format=storage,view",
            base_url.trim_end_matches('/'),
            page_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Confluence get page {}: {}", status, body));
        }
        resp.json::<ConfluencePage>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    /// Create a new page in a space
    pub async fn confluence_create_page(
        &self,
        base_url: &str,
        space_id: &str,
        title: &str,
        body_content: &str,
        parent_id: Option<&str>,
    ) -> Result<ConfluencePage, String> {
        let url = format!("{}/wiki/api/v2/pages", base_url.trim_end_matches('/'));
        let formatted_body = if body_content.starts_with('<') {
            body_content.to_string()
        } else {
            // Simple markdown-to-html paragraph wrapping
            markdown_to_storage_format(body_content)
        };

        let mut payload = serde_json::json!({
            "spaceId": space_id,
            "status": "current",
            "title": title,
            "body": {
                "representation": "storage",
                "value": formatted_body
            }
        });
        if let Some(pid) = parent_id {
            payload["parentId"] = serde_json::json!(pid);
        }

        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create page error {}: {}", status, body));
        }
        resp.json::<ConfluencePage>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    /// Update an existing page
    pub async fn confluence_update_page(
        &self,
        base_url: &str,
        page_id: &str,
        title: &str,
        body_content: &str,
        version_number: u32,
    ) -> Result<ConfluencePage, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}",
            base_url.trim_end_matches('/'),
            page_id
        );
        let formatted_body = if body_content.starts_with('<') {
            body_content.to_string()
        } else {
            markdown_to_storage_format(body_content)
        };

        let payload = serde_json::json!({
            "id": page_id,
            "status": "current",
            "title": title,
            "body": {
                "representation": "storage",
                "value": formatted_body
            },
            "version": {
                "number": version_number
            }
        });

        let resp = self.send(|| self.put(&url).json(&payload).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Update page error {}: {}", status, body));
        }
        resp.json::<ConfluencePage>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    /// Delete/archive a page
    pub async fn confluence_delete_page(
        &self,
        base_url: &str,
        page_id: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}",
            base_url.trim_end_matches('/'),
            page_id
        );
        let resp = self.send(|| self.delete(&url).send()).await?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Delete page error {}: {}", status, body))
        }
    }

    /// List footer comments for a page
    pub async fn confluence_list_comments(
        &self,
        base_url: &str,
        page_id: &str,
    ) -> Result<Vec<ConfluenceComment>, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}/footer-comments?body-format=storage,view",
            base_url.trim_end_matches('/'),
            page_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Confluence comments API {}: {}", status, body));
        }
        let data: PaginatedResults<ConfluenceComment> = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data.results)
    }

    /// Add a footer comment to a page
    pub async fn confluence_add_comment(
        &self,
        base_url: &str,
        page_id: &str,
        body_content: &str,
    ) -> Result<ConfluenceComment, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}/footer-comments",
            base_url.trim_end_matches('/'),
            page_id
        );
        let formatted_body = if body_content.starts_with('<') {
            body_content.to_string()
        } else {
            markdown_to_storage_format(body_content)
        };

        let payload = serde_json::json!({
            "body": {
                "representation": "storage",
                "value": formatted_body
            }
        });

        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Add comment error {}: {}", status, body));
        }
        resp.json::<ConfluenceComment>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    /// List version history for a page
    pub async fn confluence_list_page_versions(
        &self,
        base_url: &str,
        page_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}/versions",
            base_url.trim_end_matches('/'),
            page_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Page versions API {}: {}", status, body));
        }
        let data: PaginatedResults<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data.results)
    }

    /// List attachments for a page
    pub async fn confluence_list_attachments(
        &self,
        base_url: &str,
        page_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let url = format!(
            "{}/wiki/api/v2/pages/{}/attachments",
            base_url.trim_end_matches('/'),
            page_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Page attachments API {}: {}", status, body));
        }
        let data: PaginatedResults<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(data.results)
    }
}

pub use markdown_to_storage_format as plain_text_to_storage_format;

/// Convert simple markdown/plain text to Confluence storage format (XHTML paragraphs)
pub fn markdown_to_storage_format(input: &str) -> String {
    let mut out = String::new();
    for paragraph in input.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("### ") {
            out.push_str(&format!("<h3>{}</h3>", html_escape(heading.trim())));
        } else if let Some(heading) = trimmed.strip_prefix("## ") {
            out.push_str(&format!("<h2>{}</h2>", html_escape(heading.trim())));
        } else if let Some(heading) = trimmed.strip_prefix("# ") {
            out.push_str(&format!("<h1>{}</h1>", html_escape(heading.trim())));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            out.push_str("<ul>");
            for line in trimmed.lines() {
                let item = line.trim_start_matches(&['-', '*', ' '][..]);
                out.push_str(&format!("<li>{}</li>", html_escape(item.trim())));
            }
            out.push_str("</ul>");
        } else {
            let formatted = html_escape(trimmed).replace('\n', "<br/>");
            out.push_str(&format!("<p>{}</p>", formatted));
        }
    }
    if out.is_empty() {
        "<p></p>".to_string()
    } else {
        out
    }
}

/// Strip basic HTML tags from storage/view format for readable terminal display
pub fn storage_format_to_plain_text(html: &str) -> String {
    let without_headers = html
        .replace("<h1>", "\n# ")
        .replace("</h1>", "\n")
        .replace("<h2>", "\n## ")
        .replace("</h2>", "\n")
        .replace("<h3>", "\n### ")
        .replace("</h3>", "\n")
        .replace("<p>", "")
        .replace("</p>", "\n\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("<li>", "- ")
        .replace("</li>", "\n")
        .replace("<ul>", "\n")
        .replace("</ul>", "\n");

    let mut in_tag = false;
    let mut out = String::with_capacity(without_headers.len());
    for c in without_headers.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }

    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_to_storage_conversion() {
        let md = "# Title\n\nThis is a paragraph.\n\n- Item 1\n- Item 2";
        let storage = markdown_to_storage_format(md);
        assert!(storage.contains("<h1>Title</h1>"));
        assert!(storage.contains("<p>This is a paragraph.</p>"));
        assert!(storage.contains("<ul><li>Item 1</li><li>Item 2</li></ul>"));
    }

    #[test]
    fn storage_to_plain_text_strip() {
        let html = "<h1>Title</h1><p>Hello <strong>world</strong></p>";
        let plain = storage_format_to_plain_text(html);
        assert!(plain.contains("# Title"));
        assert!(plain.contains("Hello world"));
    }
}
