use crate::api::JiraClient;
use serde_json::Value;

impl JiraClient {
    /// List all visible projects
    pub async fn list_projects(&self, base_url: &str) -> Result<Vec<Value>, String> {
        let url = format!("{}/rest/api/3/project", base_url.trim_end_matches('/'));
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Projects API {}: {}", status, body));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// Get details of a single project by key or ID
    pub async fn get_project(&self, base_url: &str, key: &str) -> Result<Value, String> {
        let url = format!(
            "{}/rest/api/3/project/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Get project API {}: {}", status, body));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// Get valid issue types and statuses for a project
    pub async fn get_project_statuses(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<Value>, String> {
        let url = format!(
            "{}/rest/api/3/project/{}/statuses",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Project statuses API {}: {}", status, body));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// List components in a project
    pub async fn get_project_components(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<Value>, String> {
        let url = format!(
            "{}/rest/api/3/project/{}/components",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Project components API {}: {}", status, body));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// List versions in a project
    pub async fn get_project_versions(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<Value>, String> {
        let url = format!(
            "{}/rest/api/3/project/{}/versions",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Project versions API {}: {}", status, body));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }
}
