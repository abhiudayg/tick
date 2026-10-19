use crate::api::pagination;
use crate::api::types::{
    BulkFetchIssue, BulkFetchResponse, IssueDetailFields, IssueDetailResponse, JqlSearchResponse,
    Ticket,
};
use crate::api::JiraClient;
use crate::config::Config;

impl JiraClient {
    pub async fn search_jql(
        &self,
        base_url: &str,
        jql: &str,
        max_results: u32,
        progress: &mut Option<String>,
    ) -> Result<Vec<String>, String> {
        if max_results == 0 {
            return Ok(Vec::new());
        }

        let mut all_ids = Vec::new();
        let mut next_token: Option<String> = None;
        let mut page = 0u32;

        loop {
            page += 1;
            let remaining = max_results.saturating_sub(all_ids.len() as u32);
            if remaining == 0 {
                break;
            }
            let page_size = pagination::jql_page_size(remaining);

            *progress = Some(if page == 1 {
                format!("JQL search (up to {max_results})…")
            } else {
                format!("JQL search page {page} ({}/{max_results})…", all_ids.len())
            });
            tokio::task::yield_now().await;

            let url = format!("{}/rest/api/3/search/jql", base_url.trim_end_matches('/'));
            let mut body = serde_json::json!({
                "jql": jql,
                "maxResults": page_size,
            });
            if let Some(ref token) = next_token {
                body["nextPageToken"] = serde_json::Value::String(token.clone());
            }

            if self.debug {
                eprintln!(
                    "[DEBUG] POST {} (JQL: {}, page {page}, maxResults {page_size})",
                    url, jql
                );
            }

            let resp = self.send(|| self.post(&url).json(&body).send()).await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!("JQL search API {}: {}", status, body_text));
            }

            let text = resp
                .text()
                .await
                .map_err(|e| format!("Read error: {}", e))?;
            let data: JqlSearchResponse = serde_json::from_str(&text).map_err(|e| {
                format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)])
            })?;

            all_ids.extend(data.issues.into_iter().map(|i| i.id));
            if all_ids.len() as u32 >= max_results {
                all_ids.truncate(max_results as usize);
                break;
            }

            let has_more = data.next_page_token.as_ref().is_some_and(|t| !t.is_empty());
            if data.is_last || !has_more {
                break;
            }
            next_token = data.next_page_token;
        }

        Ok(all_ids)
    }

    pub async fn find_sprint_fields(
        &self,
        base_url: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!("{}/rest/api/3/field", base_url.trim_end_matches('/'));
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Field API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let list = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        let id = f["id"].as_str()?;
                        let name = f["name"].as_str()?;
                        if name.to_lowercase().contains("sprint") {
                            Some((id.to_string(), name.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(list)
    }

    pub async fn bulk_fetch(
        &self,
        base_url: &str,
        ids: &[String],
        sprint_field: Option<&str>,
        custom_field_ids: &[String],
        include_detail: bool,
    ) -> Result<Vec<BulkFetchIssue>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_issues = Vec::new();
        for chunk in ids.chunks(pagination::BULK_FETCH_CHUNK) {
            let batch = self
                .bulk_fetch_one_batch(
                    base_url,
                    chunk,
                    sprint_field,
                    custom_field_ids,
                    include_detail,
                )
                .await?;
            all_issues.extend(batch);
        }
        Ok(all_issues)
    }

    async fn bulk_fetch_one_batch(
        &self,
        base_url: &str,
        ids: &[String],
        sprint_field: Option<&str>,
        custom_field_ids: &[String],
        include_detail: bool,
    ) -> Result<Vec<BulkFetchIssue>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/rest/api/3/issue/bulkfetch",
            base_url.trim_end_matches('/')
        );
        let mut fields = vec![
            "issuetype",
            "status",
            "priority",
            "assignee",
            "reporter",
            "duedate",
            "created",
            "project",
            "summary",
            "parent",
            "labels",
        ];
        if include_detail {
            fields.push("description");
            fields.push("comment");
        }
        if let Some(sf) = sprint_field {
            if !fields.contains(&sf) {
                fields.push(sf);
            }
        }
        for cf in custom_field_ids {
            if !fields.iter().any(|f| f == cf) {
                fields.push(cf.as_str());
            }
        }

        let payload = serde_json::json!({
            "issueIdsOrKeys": ids,
            "fields": fields,
        });
        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Bulk fetch API {}: {}", status, body));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;
        let data: BulkFetchResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)]))?;

        Ok(data.issues)
    }

    /// Fetch description and comments for one issue (lazy detail pane).
    pub async fn fetch_issue_detail(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<IssueDetailFields, String> {
        let url = format!(
            "{}/rest/api/3/issue/{key}?fields=description,comment",
            base_url.trim_end_matches('/')
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Issue detail {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: IssueDetailResponse =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(data.fields)
    }
}

pub async fn fetch_all(
    client: &JiraClient,
    config: &Config,
    jql: &str,
    site_filter: Option<&str>,
    custom_field_ids: &[String],
    progress: &mut Option<String>,
) -> (Vec<Ticket>, Vec<String>) {
    let mut all = Vec::new();
    let mut errors = Vec::new();
    let cf_refs: Vec<&str> = custom_field_ids.iter().map(String::as_str).collect();

    for site in config.sites_for_fetch(site_filter) {
        if client.debug {
            eprintln!("[DEBUG] Processing site: {} ({})", site.name, site.base_url);
        }

        *progress = Some(format!("{}: searching…", site.name));
        tokio::task::yield_now().await;

        let ids = match client
            .search_jql(&site.base_url, jql, config.max_results, progress)
            .await
        {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("{}: {}", site.name, e));
                continue;
            }
        };

        *progress = Some(format!("{}: loading {} issues…", site.name, ids.len()));
        tokio::task::yield_now().await;

        let sprint_field = site.sprint_field.as_deref();
        match client
            .bulk_fetch(&site.base_url, &ids, sprint_field, custom_field_ids, false)
            .await
        {
            Ok(issues) => {
                for issue in issues {
                    all.push(Ticket::from_bulk_fetch(
                        issue,
                        &site.name,
                        &site.base_url,
                        sprint_field,
                        &cf_refs,
                        false,
                    ));
                }
            }
            Err(e) => errors.push(format!("{}: {}", site.name, e)),
        }
    }

    (all, errors)
}
