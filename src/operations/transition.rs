use std::collections::HashMap;

use crate::api::{self, transition_fields, types::WorkflowTransition, JiraClient};

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransitionSummary {
    pub id: String,
    pub name: String,
    pub to_status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransitionFieldInfo {
    pub id: String,
    pub name: String,
    pub field_type: String,
    pub kind: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransitionDetail {
    pub id: String,
    pub name: String,
    pub to_status: String,
    pub required_fields: Vec<TransitionFieldInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct TransitionOptions<'a> {
    pub resolution: Option<&'a str>,
    pub extra_fields: Option<&'a HashMap<String, serde_json::Value>>,
    pub comment: Option<&'a str>,
}

pub async fn list_transitions(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
) -> Result<Vec<TransitionSummary>, String> {
    let options = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(options
        .into_iter()
        .map(|t| TransitionSummary {
            id: t.id,
            name: t.name,
            to_status: t.to_status,
        })
        .collect())
}

pub async fn list_transitions_with_fields(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
) -> Result<Vec<TransitionDetail>, String> {
    let options = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    let pk = crate::api::types::project_key_from_issue_key(key);
    let mut details = Vec::new();

    for mut t in options {
        if transition_fields::transition_needs_detail_fetch(&t) {
            if let Ok(detail) = jira.get_transition_detail(base_url, key, &t.id).await {
                t = detail;
            }
        }
        if t.required_fields.is_empty() {
            if let Some(res) =
                transition_fields::infer_resolution_if_done_transition(&t.name, &t.to_status)
            {
                t.required_fields.push(res);
            }
        }
        api::enrich_transition_fields(jira, base_url, Some(pk), &mut t).await;

        let req_fields = t
            .required_fields
            .into_iter()
            .map(|f| {
                let options = f.options.into_iter().map(|(_, label)| label).collect();
                TransitionFieldInfo {
                    id: f.id,
                    name: f.name,
                    field_type: f.field_type,
                    kind: format!("{:?}", f.kind),
                    options,
                }
            })
            .collect();

        details.push(TransitionDetail {
            id: t.id,
            name: t.name,
            to_status: t.to_status,
            required_fields: req_fields,
        });
    }

    Ok(details)
}

pub async fn apply_transition_by_name(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
    transition_name: &str,
) -> Result<(), String> {
    apply_transition_by_name_with_options(
        jira,
        base_url,
        key,
        transition_name,
        &TransitionOptions::default(),
    )
    .await
}

pub async fn apply_transition_by_id(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
    transition_id: &str,
) -> Result<(), String> {
    apply_transition_by_id_with_options(
        jira,
        base_url,
        key,
        transition_id,
        &TransitionOptions::default(),
    )
    .await
}

pub async fn apply_transition_by_name_with_options(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
    transition_name: &str,
    options: &TransitionOptions<'_>,
) -> Result<(), String> {
    let transitions = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;

    let Some(transition) = transitions.into_iter().find(|t| {
        t.name.eq_ignore_ascii_case(transition_name)
            || t.to_status.eq_ignore_ascii_case(transition_name)
            || t.id == transition_name
    }) else {
        return Err(format!("no transition matching '{transition_name}'"));
    };

    apply_prepared_transition(jira, base_url, key, transition, options).await
}

pub async fn apply_transition_by_id_with_options(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
    transition_id: &str,
    options: &TransitionOptions<'_>,
) -> Result<(), String> {
    let transitions = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    let Some(transition) = transitions.into_iter().find(|t| t.id == transition_id) else {
        return Err(format!("no transition id '{transition_id}'"));
    };
    apply_prepared_transition(jira, base_url, key, transition, options).await
}

async fn apply_prepared_transition(
    jira: &JiraClient,
    base_url: &str,
    key: &str,
    mut transition: WorkflowTransition,
    options: &TransitionOptions<'_>,
) -> Result<(), String> {
    if transition_fields::transition_needs_detail_fetch(&transition) {
        if let Ok(detail) = jira
            .get_transition_detail(base_url, key, &transition.id)
            .await
        {
            transition = detail;
        }
    }

    if transition.required_fields.is_empty() {
        if let Some(res) = transition_fields::infer_resolution_if_done_transition(
            &transition.name,
            &transition.to_status,
        ) {
            transition.required_fields.push(res);
        }
    }

    let pk = crate::api::types::project_key_from_issue_key(key);
    api::enrich_transition_fields(jira, base_url, Some(pk), &mut transition).await;

    let mut fields: HashMap<String, serde_json::Value> = HashMap::new();

    if let Some(res) = options.resolution {
        fields.insert("resolution".to_string(), serde_json::json!({ "name": res }));
    }

    if let Some(extra) = options.extra_fields {
        for (k, v) in extra {
            fields.insert(k.clone(), v.clone());
        }
    }

    // Check missing required fields
    let missing: Vec<&crate::api::transition_fields::TransitionField> = transition
        .required_fields
        .iter()
        .filter(|rf| !fields.contains_key(&rf.id))
        .collect();

    if !missing.is_empty() {
        let mut err_msg = format!(
            "Transition \"{}\" requires missing field(s):\n",
            transition.name
        );
        for f in &missing {
            if !f.options.is_empty() {
                let opt_names: Vec<&str> = f.options.iter().map(|(_, l)| l.as_str()).collect();
                err_msg.push_str(&format!(
                    "  • {} (id: {}) — options: [{}]\n",
                    f.name,
                    f.id,
                    opt_names.join(", ")
                ));
            } else {
                err_msg.push_str(&format!("  • {} (id: {})\n", f.name, f.id));
            }
        }
        err_msg.push_str(
            "Provide them using --resolution <name>, -F <key>=<value>, or --fields '<json>'.",
        );
        return Err(err_msg);
    }

    jira.transition_issue(base_url, key, &transition, &fields)
        .await
        .map_err(|e| e.message)?;

    if let Some(comment) = options.comment {
        if !comment.trim().is_empty() {
            let _ = jira
                .add_comment_with_attachments(base_url, key, comment, &[], &[])
                .await;
        }
    }

    Ok(())
}
