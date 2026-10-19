pub mod adf;
pub mod adf_export;
pub mod agile;
pub mod assignable_users;
pub mod attachments;
pub mod comments;
pub mod confluence;
pub mod create;
pub mod fetch;
pub mod fields;
pub mod issue_relations;
pub mod issue_updates;
pub mod jira_error;
pub mod markdown;
pub mod pagination;
pub mod projects;
pub mod retry;
pub mod transition_fields;
pub mod transitions;
pub mod types;

#[cfg(test)]
mod fetch_tests;
#[cfg(test)]
mod field_tests;

pub use create::{CreateDraft, CreateError};
pub use fetch::fetch_all;
pub use transitions::enrich_transition_fields;

use crate::auth::Auth;
use crate::config::Config;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Mutex;

/// Failed transition POST; `field_errors` lists Jira `errors` keys when present.
#[derive(Debug, Clone)]
pub struct TransitionError {
    pub message: String,
    pub field_errors: Vec<(String, String)>,
}

pub struct JiraClient {
    pub http: Client,
    pub(crate) auth: Auth,
    pub debug: bool,
    pub(crate) account_ids: Mutex<HashMap<String, String>>,
    pub(crate) priorities: Mutex<HashMap<String, Vec<(String, String)>>>,
    pub(crate) resolutions: Mutex<HashMap<String, Vec<(String, String)>>>,
    /// Full assignable-user catalog per `base_url|issue_key`.
    pub(crate) assignable_users: Mutex<HashMap<String, Vec<(String, String)>>>,
}

impl JiraClient {
    pub fn new(email: &str, token: &str, debug: bool) -> Self {
        Self::with_auth(Auth::basic(email, token), debug)
    }

    pub fn with_auth(auth: Auth, debug: bool) -> Self {
        Self {
            http: Client::new(),
            auth,
            debug,
            account_ids: Mutex::new(HashMap::new()),
            priorities: Mutex::new(HashMap::new()),
            resolutions: Mutex::new(HashMap::new()),
            assignable_users: Mutex::new(HashMap::new()),
        }
    }

    pub async fn from_config(config: &Config, debug: bool) -> Result<Self, String> {
        let auth = config.resolve_auth().await?;
        Ok(Self::with_auth(auth, debug))
    }

    pub fn email(&self) -> &str {
        self.auth.email()
    }

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        self.auth.apply(req)
    }

    pub(crate) fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.get(url))
    }

    pub(crate) fn put(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.put(url))
    }

    pub(crate) fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.post(url))
    }

    pub(crate) fn delete(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.delete(url))
    }

    pub(crate) async fn send<F, Fut>(&self, build: F) -> Result<reqwest::Response, String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        retry::with_retry(build).await
    }

    /// Generic authenticated request for arbitrary Jira or Confluence REST endpoints
    pub async fn request_raw(
        &self,
        base_url: &str,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<reqwest::Response, String> {
        let trimmed_ep = endpoint.trim_start_matches('/');
        let url = if trimmed_ep.starts_with("wiki/") || trimmed_ep.starts_with("rest/") {
            format!("{}/{}", base_url.trim_end_matches('/'), trimmed_ep)
        } else {
            format!(
                "{}/rest/api/3/{}",
                base_url.trim_end_matches('/'),
                trimmed_ep
            )
        };
        self.send(|| {
            let mut req = self.authed(self.http.request(method.clone(), &url));
            if let Some(b) = body {
                req = req.json(b);
            }
            req.send()
        })
        .await
    }
}
