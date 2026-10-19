use crate::api::JiraClient;
use crate::config::Config;
use clap::Parser;
use reqwest::Method;
use serde_json::Value;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct ApiArgs {
    /// The Atlassian REST API endpoint to call (e.g. "/rest/api/3/myself" or "/wiki/api/v2/spaces")
    pub endpoint: String,

    /// HTTP method (GET, POST, PUT, DELETE, PATCH)
    #[arg(short = 'X', long, default_value = "GET")]
    pub method: String,

    /// JSON request body string
    #[arg(short = 'b', long)]
    pub body: Option<String>,

    /// Target site name
    #[arg(long)]
    pub site: Option<String>,
}

pub async fn run(args: ApiArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let site = if let Some(name) = args.site.as_deref() {
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

    let method = Method::from_str(&args.method.to_uppercase())
        .map_err(|e| format!("Invalid HTTP method: {e}"))?;

    let body_json = if let Some(ref b) = args.body {
        let val: Value =
            serde_json::from_str(b).map_err(|e| format!("Invalid JSON in request body: {e}"))?;
        Some(val)
    } else {
        None
    };

    let resp = client
        .request_raw(&site.base_url, method, &args.endpoint, body_json.as_ref())
        .await?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;
    if let Ok(val) = serde_json::from_str::<Value>(&text) {
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
    } else {
        println!("{text}");
    }

    if !status.is_success() {
        return Err(format!("HTTP error {status}"));
    }
    Ok(())
}
