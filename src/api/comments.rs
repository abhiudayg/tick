use crate::api::JiraClient;
use std::path::PathBuf;

impl JiraClient {
    pub async fn add_comment(
        &self,
        base_url: &str,
        key: &str,
        body: &str,
        mentions: &[(String, String)],
    ) -> Result<(), String> {
        self.add_comment_with_attachments(base_url, key, body, mentions, &[])
            .await
    }

    pub async fn add_comment_with_attachments(
        &self,
        base_url: &str,
        key: &str,
        body: &str,
        mentions: &[(String, String)],
        attach_paths: &[PathBuf],
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment",
            base_url.trim_end_matches('/'),
            key
        );
        let adf_body = self
            .build_comment_body(base_url, key, body, mentions, attach_paths)
            .await?;
        let payload = serde_json::json!({ "body": adf_body });
        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Comment API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn add_worklog(
        &self,
        base_url: &str,
        key: &str,
        time_spent: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/worklog",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.post(&url)
                    .json(&serde_json::json!({ "timeSpent": time_spent }))
                    .send()
            })
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Worklog API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn list_comments(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "List comments API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(data
            .get("comments")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn delete_comment(
        &self,
        base_url: &str,
        key: &str,
        comment_id: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment/{}",
            base_url.trim_end_matches('/'),
            key,
            comment_id
        );
        let resp = self.send(|| self.delete(&url).send()).await?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(format!(
                "Delete comment {comment_id} failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn list_worklogs(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/worklog",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "List worklogs API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(data
            .get("worklogs")
            .and_then(|w| w.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn delete_worklog(
        &self,
        base_url: &str,
        key: &str,
        worklog_id: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/worklog/{}",
            base_url.trim_end_matches('/'),
            key,
            worklog_id
        );
        let resp = self.send(|| self.delete(&url).send()).await?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(format!(
                "Delete worklog {worklog_id} failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }
}
