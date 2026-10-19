use crate::api::assignable_users;
use crate::api::markdown;
use crate::api::JiraClient;
use crate::config::Site;
use reqwest::StatusCode;

impl JiraClient {
    pub async fn list_priorities(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.priorities.lock() {
            if let Some(list) = cache.get(base) {
                return Ok(list.clone());
            }
        }
        let url = format!("{base}/rest/api/3/priority");
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Priority API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let list: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let id = p["id"].as_str()?;
                        let name = p["name"].as_str()?;
                        Some((id.to_string(), name.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(mut cache) = self.priorities.lock() {
            cache.insert(base.to_string(), list.clone());
        }
        Ok(list)
    }

    pub async fn update_summary(
        &self,
        base_url: &str,
        key: &str,
        summary: &str,
    ) -> Result<(), String> {
        self.update_fields(base_url, key, serde_json::json!({ "summary": summary }))
            .await
    }

    pub async fn update_priority(
        &self,
        base_url: &str,
        key: &str,
        priority_name: &str,
    ) -> Result<(), String> {
        self.update_fields(
            base_url,
            key,
            serde_json::json!({ "priority": { "name": priority_name } }),
        )
        .await
    }

    pub async fn update_labels(
        &self,
        base_url: &str,
        key: &str,
        labels: &[String],
    ) -> Result<(), String> {
        self.update_fields(base_url, key, serde_json::json!({ "labels": labels }))
            .await
    }

    pub async fn update_description(
        &self,
        base_url: &str,
        key: &str,
        text: &str,
        mentions: &[(String, String)],
    ) -> Result<(), String> {
        let body = markdown::to_adf(text, mentions);
        self.update_fields(base_url, key, serde_json::json!({ "description": body }))
            .await
    }

    /// Set or clear due date (`YYYY-MM-DD`). `None` clears the field.
    pub async fn update_due_date(
        &self,
        base_url: &str,
        key: &str,
        due_date: Option<chrono::NaiveDate>,
    ) -> Result<(), String> {
        let value = match due_date {
            Some(d) => serde_json::json!(d.format("%Y-%m-%d").to_string()),
            None => serde_json::Value::Null,
        };
        self.update_fields(base_url, key, serde_json::json!({ "duedate": value }))
            .await
    }

    /// Add the current user as a watcher on the issue.
    pub async fn watch_issue(&self, base_url: &str, key: &str) -> Result<(), String> {
        let account_id = self.current_user_account_id(base_url).await?;
        let url = format!(
            "{}/rest/api/3/issue/{}/watchers",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.post(&url)
                    .json(&serde_json::json!({ "accountId": account_id }))
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Watch failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    /// Remove the current user from watchers.
    pub async fn unwatch_issue(&self, base_url: &str, key: &str) -> Result<(), String> {
        let account_id = self.current_user_account_id(base_url).await?;
        let url = format!(
            "{}/rest/api/3/issue/{}/watchers",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.delete(&url)
                    .query(&[("accountId", account_id.as_str())])
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Unwatch failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    /// Update a single issue field (standard or `customfield_*`).
    pub async fn update_issue_field(
        &self,
        base_url: &str,
        key: &str,
        field_id: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let fields = serde_json::json!({ field_id: value });
        self.update_fields(base_url, key, fields).await
    }

    async fn update_fields(
        &self,
        base_url: &str,
        key: &str,
        fields: serde_json::Value,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.put(&url)
                    .json(&serde_json::json!({ "fields": fields }))
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Update failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    /// `GET /rest/api/3/issue/{key}` — true when the issue exists (or is forbidden but present).
    pub async fn issue_exists(&self, base_url: &str, key: &str) -> bool {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/issue/{key}?fields=key");
        let resp = match self.send(|| self.get(&url).send()).await {
            Ok(r) => r,
            Err(_) => return false,
        };
        let status = resp.status();
        status.is_success() || status == StatusCode::FORBIDDEN
    }

    /// Probe sites in config order; return browse URL for the first match.
    pub async fn find_issue_browse_url(&self, sites: &[Site], key: &str) -> Option<String> {
        for site in sites {
            if self.issue_exists(&site.base_url, key).await {
                let base = site.base_url.trim_end_matches('/');
                return Some(format!("{base}/browse/{key}"));
            }
        }
        None
    }

    pub async fn current_user_account_id(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.account_ids.lock() {
            if let Some(id) = cache.get(base) {
                return Ok(id.clone());
            }
        }
        let url = format!("{base}/rest/api/3/myself");
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Myself API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let id = body
            .get("accountId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Myself API: missing accountId".to_string())?
            .to_string();
        if let Ok(mut cache) = self.account_ids.lock() {
            cache.insert(base.to_string(), id.clone());
        }
        Ok(id)
    }

    pub async fn assign_to_account(
        &self,
        base_url: &str,
        key: &str,
        account_id: &str,
    ) -> Result<(), String> {
        self.set_assignee(
            base_url,
            key,
            Some(serde_json::json!({ "accountId": account_id })),
        )
        .await
    }

    pub async fn unassign(&self, base_url: &str, key: &str) -> Result<(), String> {
        self.set_assignee(base_url, key, None).await
    }

    async fn set_assignee(
        &self,
        base_url: &str,
        key: &str,
        assignee: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let payload = serde_json::json!({ "fields": { "assignee": assignee } });
        let resp = self.send(|| self.put(&url).json(&payload).send()).await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Assign failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    async fn fetch_assignable_users_api(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
        max_results: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!(
            "{}/rest/api/3/user/assignable/search",
            base_url.trim_end_matches('/')
        );
        let project_only = issue_key.strip_suffix(".__CREATE__");
        let resp = self
            .send(|| {
                let mut req = self
                    .get(&url)
                    .query(&[("query", query), ("maxResults", max_results)]);
                if let Some(project) = project_only {
                    req = req.query(&[("project", project)]);
                } else {
                    req = req.query(&[("issueKey", issue_key)]);
                }
                req.send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "User search {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(parse_user_list(&data))
    }

    /// Load assignable users for an issue into the session cache.
    ///
    /// Initial load uses `api_query` `""` (up to 100). On refresh, fetches with `api_query`
    /// (often the footer filter) and **merges** into the existing catalog.
    pub async fn ensure_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        api_query: &str,
        force_refresh: bool,
    ) -> Result<Vec<(String, String)>, String> {
        let key = assignable_users::cache_key(base_url, issue_key);
        let existing = if force_refresh {
            self.assignable_users
                .lock()
                .ok()
                .and_then(|c| c.get(&key).cloned())
                .unwrap_or_default()
        } else if let Ok(cache) = self.assignable_users.lock() {
            if let Some(list) = cache.get(&key) {
                return Ok(list.clone());
            }
            Vec::new()
        } else {
            Vec::new()
        };

        let fetched = self
            .fetch_assignable_users_api(
                base_url,
                issue_key,
                api_query,
                assignable_users::CATALOG_MAX,
            )
            .await?;

        let list = if force_refresh {
            assignable_users::merge_users(&existing, &fetched)
        } else {
            fetched
        };

        if let Ok(mut cache) = self.assignable_users.lock() {
            cache.insert(key, list.clone());
        }
        Ok(list)
    }

    /// Filter cached assignable users (loads catalog on first use).
    pub async fn filter_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let catalog = self
            .ensure_assignable_users(base_url, issue_key, "", false)
            .await?;
        Ok(assignable_users::filter_users(&catalog, query))
    }

    pub async fn search_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
    ) -> Result<Vec<(String, String)>, String> {
        self.filter_assignable_users(base_url, issue_key, query)
            .await
    }

    pub async fn delete_issue(&self, base_url: &str, key: &str) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.delete(&url).send()).await?;
        if resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(format!(
                "Delete issue {key} failed ({}): {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn get_user_profile(&self, base_url: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/rest/api/3/myself", base_url.trim_end_matches('/'));
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Myself API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    pub async fn search_users(
        &self,
        base_url: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let lim = limit.unwrap_or(25).to_string();
        let url = format!("{}/rest/api/3/user/search", base_url.trim_end_matches('/'));
        let resp = self
            .send(|| {
                self.get(&url)
                    .query(&[("query", query), ("maxResults", &lim)])
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "User search {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }
}

fn parse_user_list(data: &serde_json::Value) -> Vec<(String, String)> {
    data.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|u| {
                    let id = u["accountId"].as_str()?;
                    let name = u["displayName"].as_str()?;
                    Some((id.to_string(), name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}
