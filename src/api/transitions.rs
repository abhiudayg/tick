use crate::api::jira_error;
use crate::api::transition_fields;
use crate::api::types;
use crate::api::JiraClient;
use crate::api::TransitionError;
use std::collections::HashMap;

impl JiraClient {
    /// List workflow transitions available for the issue (status changes go through these).
    pub async fn get_workflow_transitions(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<types::WorkflowTransition>, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions?expand=transitions.fields",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(jira_error::format_response_error(status, &body));
        }

        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse transitions: {e}"))?;
        Ok(parse_workflow_transitions(&data))
    }

    /// Fetch one transition with full screen field metadata (Jira often omits fields on the list call).
    pub async fn get_transition_detail(
        &self,
        base_url: &str,
        key: &str,
        transition_id: &str,
    ) -> Result<types::WorkflowTransition, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions?transitionId={}&expand=transitions.fields",
            base_url.trim_end_matches('/'),
            key,
            transition_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(jira_error::format_response_error(status, &body));
        }
        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse transition detail: {e}"))?;
        parse_single_transition(&data)
            .ok_or_else(|| format!("Transition {transition_id} not returned by Jira for {key}"))
    }

    /// Project resolutions (cached per site for the session).
    pub async fn list_resolutions(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.resolutions.lock() {
            if let Some(list) = cache.get(base) {
                return Ok(list.clone());
            }
        }
        let url = format!("{base}/rest/api/3/resolution");
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Resolution list {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let list: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        let id = r["id"].as_str()?.to_string();
                        let name = r["name"].as_str()?.to_string();
                        Some((id, name))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(mut cache) = self.resolutions.lock() {
            cache.insert(base.to_string(), list.clone());
        }
        Ok(list)
    }

    /// Preload resolution + priority catalogs (parallel, cached) before field prompts.
    pub async fn warm_site_field_catalogs(&self, base_url: &str) {
        let base = base_url.trim_end_matches('/');
        let need_res = self
            .resolutions
            .lock()
            .ok()
            .is_none_or(|c| !c.contains_key(base));
        let need_pri = self
            .priorities
            .lock()
            .ok()
            .is_none_or(|c| !c.contains_key(base));
        if !need_res && !need_pri {
            return;
        }
        let (res, pri) = tokio::join!(
            async {
                if need_res {
                    self.list_resolutions(base_url).await.ok()
                } else {
                    None
                }
            },
            async {
                if need_pri {
                    self.list_priorities(base_url).await.ok()
                } else {
                    None
                }
            }
        );
        let _ = (res, pri);
    }

    pub async fn transition_issue(
        &self,
        base_url: &str,
        key: &str,
        transition: &types::WorkflowTransition,
        fields: &HashMap<String, serde_json::Value>,
    ) -> Result<(), TransitionError> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            base_url.trim_end_matches('/'),
            key
        );
        let mut payload = serde_json::json!({ "transition": { "id": transition.id } });
        if !fields.is_empty() {
            payload["fields"] = serde_json::json!(fields);
        }
        let resp = self
            .send(|| self.post(&url).json(&payload).send())
            .await
            .map_err(|e| TransitionError {
                message: e,
                field_errors: Vec::new(),
            })?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        let detail = jira_error::format_response_error(status, &body);
        let label = if transition.to_status.is_empty() {
            transition.name.clone()
        } else {
            format!("{} → {}", transition.name, transition.to_status)
        };
        Err(TransitionError {
            message: format!("Cannot apply \"{label}\": {detail}"),
            field_errors: jira_error::field_errors(&body),
        })
    }
}

pub(crate) fn parse_workflow_transitions(
    data: &serde_json::Value,
) -> Vec<types::WorkflowTransition> {
    data["transitions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let id = t["id"].as_str()?.to_string();
                    let name = t["name"].as_str()?.to_string();
                    let to_status = t
                        .get("to")
                        .and_then(|to| to.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let required_fields =
                        transition_fields::parse_transition_screen_fields(t.get("fields"));
                    Some(types::WorkflowTransition {
                        id,
                        name,
                        to_status,
                        required_fields,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_single_transition(
    data: &serde_json::Value,
) -> Option<types::WorkflowTransition> {
    data["transitions"].as_array()?.first().and_then(|t| {
        let id = t["id"].as_str()?.to_string();
        let name = t["name"].as_str()?.to_string();
        let to_status = t
            .get("to")
            .and_then(|to| to.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let mut required_fields =
            transition_fields::parse_transition_screen_fields(t.get("fields"));
        if required_fields.is_empty() {
            if let Some(res) =
                transition_fields::infer_resolution_if_done_transition(&name, &to_status)
            {
                required_fields.push(res);
            }
        }
        Some(types::WorkflowTransition {
            id,
            name,
            to_status,
            required_fields,
        })
    })
}

pub async fn enrich_transition_fields(
    client: &JiraClient,
    base_url: &str,
    project_key: Option<&str>,
    transition: &mut types::WorkflowTransition,
) {
    let needs_resolution = transition
        .required_fields
        .iter()
        .any(|f| f.id == "resolution" && f.options.is_empty());
    let needs_priority = transition.required_fields.iter().any(|f| {
        (f.id == "priority" || f.system == "priority")
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::Picker
    });
    let needs_components = transition.required_fields.iter().any(|f| {
        f.id == "components"
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::MultiPicker
    });
    let needs_versions = transition.required_fields.iter().any(|f| {
        f.id == "fixVersions"
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::MultiPicker
    });
    if !needs_resolution && !needs_priority && !needs_components && !needs_versions {
        return;
    }

    let pk = project_key.unwrap_or("");
    let (resolutions, priorities, components, versions) = tokio::join!(
        async {
            if needs_resolution {
                client.list_resolutions(base_url).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_priority {
                client.list_priorities(base_url).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_components && !pk.is_empty() {
                client.list_project_components(base_url, pk).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_versions && !pk.is_empty() {
                client.list_project_versions(base_url, pk).await.ok()
            } else {
                None
            }
        },
    );

    if let Some(opts) = resolutions {
        for field in &mut transition.required_fields {
            if field.id == "resolution" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = priorities {
        for field in &mut transition.required_fields {
            if (field.id == "priority" || field.system == "priority") && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = components {
        for field in &mut transition.required_fields {
            if field.id == "components" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = versions {
        for field in &mut transition.required_fields {
            if field.id == "fixVersions" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
}
