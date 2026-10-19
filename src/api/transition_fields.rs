//! Parse required transition fields and build Jira `fields` payloads.

use chrono::NaiveDate;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionFieldKind {
    /// `allowedValues` or loaded options (resolution, priority, select list).
    Picker,
    /// `components`, `fixVersions` — Space toggles, Enter submits array of ids.
    MultiPicker,
    User,
    Boolean,
    Date,
    DateTime,
    Number,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionField {
    pub id: String,
    pub name: String,
    pub field_type: String,
    pub system: String,
    pub kind: TransitionFieldKind,
    /// `(id, label)` from Jira `allowedValues`, when present.
    pub options: Vec<(String, String)>,
}

impl TransitionField {
    pub fn input_hint(&self) -> &'static str {
        match self.kind {
            TransitionFieldKind::User => "type to filter users",
            TransitionFieldKind::Date => "YYYY-MM-DD",
            TransitionFieldKind::DateTime => "YYYY-MM-DD or YYYY-MM-DDTHH:MM",
            TransitionFieldKind::Boolean => "choose Yes or No",
            TransitionFieldKind::Number => "number",
            TransitionFieldKind::Text => "text",
            TransitionFieldKind::Picker => "choose an option",
            TransitionFieldKind::MultiPicker => "Space toggles · Enter confirms",
        }
    }

    pub fn modal_hint(&self) -> &'static str {
        match self.kind {
            TransitionFieldKind::User => "Select an assignable user (type to search)",
            TransitionFieldKind::Date => "Enter a date (YYYY-MM-DD)",
            TransitionFieldKind::DateTime => "Enter date/time in the footer, then Enter",
            TransitionFieldKind::Boolean => "Select Yes or No",
            TransitionFieldKind::Number => "Enter a number in the footer, then Enter",
            TransitionFieldKind::Text => "Enter text in the footer below, then Enter",
            TransitionFieldKind::Picker => "Select an option",
            TransitionFieldKind::MultiPicker => {
                "Space toggles options · Enter confirms (Esc cancels)"
            }
        }
    }

    pub fn value_from_multi_choices(&self, picks: &[(String, String)]) -> Value {
        let items: Vec<Value> = picks.iter().map(|(id, _)| json!({ "id": id })).collect();
        json!(items)
    }

    pub fn value_from_choice(&self, id: &str, label: &str) -> Value {
        match self.kind {
            TransitionFieldKind::User => json!({ "accountId": id }),
            TransitionFieldKind::Boolean => json!(id == "true"),
            TransitionFieldKind::Picker => self.picker_value(id, label),
            _ => self.picker_value(id, label),
        }
    }

    fn picker_value(&self, id: &str, label: &str) -> Value {
        match self.field_type.as_str() {
            "resolution" => json!({ "name": label }),
            "priority" => json!({ "id": id }),
            "option" => json!({ "id": id }),
            _ if self.id == "resolution" => json!({ "name": label }),
            _ if self.system == "priority" => json!({ "id": id }),
            _ => json!({ "id": id }),
        }
    }

    /// Build JSON for footer text entry; returns `Err` with a short validation message.
    pub fn value_from_text(&self, text: &str) -> Result<Value, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(format!("{} cannot be empty", self.name));
        }
        match self.kind {
            TransitionFieldKind::Text | TransitionFieldKind::Number => {}
            _ => {}
        }
        Ok(match self.kind {
            TransitionFieldKind::Text => match self.field_type.as_str() {
                "string" => json!(trimmed),
                _ => json!({ "value": trimmed }),
            },
            TransitionFieldKind::Number => trimmed
                .parse::<f64>()
                .map(|n| json!(n))
                .map_err(|_| format!("{} must be a number", self.name))?,
            TransitionFieldKind::Date => {
                let d = parse_date(trimmed)?;
                json!(d)
            }
            TransitionFieldKind::DateTime => {
                let dt = parse_datetime(trimmed)?;
                json!(dt)
            }
            TransitionFieldKind::User => {
                return Err(format!(
                    "Select a user for {} from the list (type to search)",
                    self.name
                ));
            }
            TransitionFieldKind::Boolean | TransitionFieldKind::Picker => {
                return Err(format!("Select a value for {} from the list", self.name));
            }
            TransitionFieldKind::MultiPicker => {
                return Err(format!(
                    "Select one or more values for {} from the checklist",
                    self.name
                ));
            }
        })
    }
}

fn parse_date(s: &str) -> Result<String, String> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.format("%Y-%m-%d").to_string());
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d/%m/%Y") {
        return Ok(d.format("%Y-%m-%d").to_string());
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Ok(d.format("%Y-%m-%d").to_string());
    }
    Err("Use date format YYYY-MM-DD".into())
}

fn parse_datetime(s: &str) -> Result<String, String> {
    if s.contains('T') {
        return Ok(s.to_string());
    }
    let date = parse_date(s)?;
    Ok(format!("{date}T12:00:00.000+0000"))
}

pub const BOOLEAN_OPTIONS: [(&str, &str); 2] = [("true", "Yes"), ("false", "No")];

fn classify_field(
    id: &str,
    field_type: &str,
    system: &str,
    custom: &str,
    options: &[(String, String)],
) -> TransitionFieldKind {
    if matches!(id, "components" | "fixVersions")
        || matches!(system, "components" | "fixVersions")
        || (field_type == "array" && matches!(system, "components" | "fixVersions"))
    {
        return TransitionFieldKind::MultiPicker;
    }
    if field_type == "array"
        && (custom.contains("multiselect")
            || custom.contains("checkbox")
            || custom.contains("multicheckboxes"))
    {
        return TransitionFieldKind::MultiPicker;
    }
    if !options.is_empty() {
        if field_type == "boolean" {
            return TransitionFieldKind::Boolean;
        }
        return TransitionFieldKind::Picker;
    }
    match field_type {
        "user" => return TransitionFieldKind::User,
        "date" => return TransitionFieldKind::Date,
        "datetime" => return TransitionFieldKind::DateTime,
        "boolean" => return TransitionFieldKind::Boolean,
        "number" => return TransitionFieldKind::Number,
        "string" => return TransitionFieldKind::Text,
        "priority" | "resolution" | "option" => return TransitionFieldKind::Picker,
        _ => {}
    }
    match system {
        "assignee" | "reporter" => TransitionFieldKind::User,
        "fixVersions" | "components" => TransitionFieldKind::MultiPicker,
        "resolution" | "priority" => TransitionFieldKind::Picker,
        _ => TransitionFieldKind::Text,
    }
}

/// Required fields on the create issue screen (excludes project, type, summary, description).
pub fn parse_create_fields(fields_obj: &Value) -> Vec<TransitionField> {
    let skip = [
        "project",
        "issuetype",
        "summary",
        "description",
        "reporter",
        "attachment",
    ];
    let mut all = parse_transition_screen_fields(Some(fields_obj));
    all.retain(|f| {
        !skip.iter().any(|s| f.id == *s || f.system == *s)
            && fields_obj
                .get(&f.id)
                .and_then(|m| m.get("required"))
                .and_then(|r| r.as_bool())
                .unwrap_or(false)
    });
    all
}

/// Fields on the transition screen that need user input before POST.
pub fn parse_transition_screen_fields(fields_obj: Option<&Value>) -> Vec<TransitionField> {
    let Some(obj) = fields_obj.and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out: Vec<TransitionField> = obj
        .iter()
        .filter_map(|(id, meta)| {
            if !field_needs_input(meta) {
                return None;
            }
            let name = meta
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(id)
                .to_string();
            let schema = meta.get("schema");
            let field_type = schema
                .and_then(|s| s.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let system = schema
                .and_then(|s| s.get("system"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let custom = schema
                .and_then(|s| s.get("custom"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let mut options = meta
                .get("allowedValues")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(parse_allowed_value)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let kind = classify_field(id, &field_type, &system, &custom, &options);
            if kind == TransitionFieldKind::Boolean && options.is_empty() {
                options = BOOLEAN_OPTIONS
                    .iter()
                    .map(|(id, label)| (id.to_string(), label.to_string()))
                    .collect();
            }
            Some(TransitionField {
                id: id.clone(),
                name,
                field_type,
                system,
                kind,
                options,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn field_needs_input(meta: &Value) -> bool {
    if meta.get("required").and_then(|r| r.as_bool()) == Some(true) {
        return true;
    }
    if meta
        .get("allowedValues")
        .and_then(|a| a.as_array())
        .is_some_and(|a| !a.is_empty())
    {
        return true;
    }
    let field_type = meta
        .get("schema")
        .and_then(|s| s.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let system = meta
        .get("schema")
        .and_then(|s| s.get("system"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    matches!(
        field_type,
        "resolution"
            | "priority"
            | "user"
            | "option"
            | "array"
            | "version"
            | "component"
            | "date"
            | "datetime"
            | "boolean"
            | "number"
            | "string"
    ) || matches!(
        system,
        "resolution" | "priority" | "assignee" | "reporter" | "fixVersions" | "components"
    )
}

/// When Jira omits `fields` on the list response, still prompt for resolution on Done/Close transitions.
/// Extra GET with `transitionId` only when the list response had no field metadata.
pub fn transition_needs_detail_fetch(transition: &crate::api::types::WorkflowTransition) -> bool {
    transition.required_fields.is_empty()
}

pub fn infer_resolution_if_done_transition(
    transition_name: &str,
    to_status: &str,
) -> Option<TransitionField> {
    let done_like = |s: &str| {
        let s = s.to_lowercase();
        s.contains("done")
            || s.contains("closed")
            || s.contains("close")
            || s.contains("complete")
            || s.contains("resolved")
    };
    if done_like(transition_name) || done_like(to_status) {
        Some(TransitionField {
            id: "resolution".into(),
            name: "Resolution".into(),
            field_type: "resolution".into(),
            system: "resolution".into(),
            kind: TransitionFieldKind::Picker,
            options: Vec::new(),
        })
    } else {
        None
    }
}

fn parse_allowed_value(v: &Value) -> Option<(String, String)> {
    if let Some(id) = v.get("id") {
        let id_s = id
            .as_str()
            .map(String::from)
            .or_else(|| id.as_i64().map(|n| n.to_string()))?;
        let label = v
            .get("name")
            .or_else(|| v.get("value"))
            .and_then(|l| l.as_str())
            .unwrap_or(&id_s)
            .to_string();
        return Some((id_s, label));
    }
    if let Some(aid) = v.get("accountId").and_then(|a| a.as_str()) {
        let label = v
            .get("displayName")
            .and_then(|d| d.as_str())
            .unwrap_or(aid)
            .to_string();
        return Some((aid.to_string(), label));
    }
    None
}

pub fn field_for_error_key(key: &str, known: &[TransitionField]) -> TransitionField {
    known
        .iter()
        .find(|f| f.id == key)
        .cloned()
        .unwrap_or_else(|| infer_field_from_id(key))
}

fn infer_field_from_id(key: &str) -> TransitionField {
    let (kind, field_type, system) = match key {
        "resolution" => (TransitionFieldKind::Picker, "resolution", "resolution"),
        "assignee" | "reporter" => (TransitionFieldKind::User, "user", key),
        "priority" => (TransitionFieldKind::Picker, "priority", "priority"),
        "fixVersions" | "components" => (TransitionFieldKind::MultiPicker, "array", key),
        _ if key.contains("date") || key.ends_with("Date") => {
            (TransitionFieldKind::Date, "date", "")
        }
        _ => (TransitionFieldKind::Text, "string", ""),
    };
    let mut options = Vec::new();
    if kind == TransitionFieldKind::Boolean {
        options = BOOLEAN_OPTIONS
            .iter()
            .map(|(id, label)| (id.to_string(), label.to_string()))
            .collect();
    }
    TransitionField {
        id: key.to_string(),
        name: humanize_field_id(key),
        field_type: field_type.into(),
        system: system.into(),
        kind,
        options,
    }
}

fn humanize_field_id(id: &str) -> String {
    match id {
        "resolution" => "Resolution".into(),
        "assignee" => "Assignee".into(),
        "reporter" => "Reporter".into(),
        "fixVersions" => "Fix versions".into(),
        "components" => "Components".into(),
        other => {
            let mut s = other.replace("customfield_", "Custom field ");
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_resolution_with_options() {
        let fields = json!({
            "resolution": {
                "required": true,
                "name": "Resolution",
                "schema": { "type": "resolution", "system": "resolution" },
                "allowedValues": [
                    { "id": "10000", "name": "Done" },
                    { "id": "10001", "name": "Won't Do" }
                ]
            },
            "summary": { "required": false, "name": "Summary" }
        });
        let req = parse_transition_screen_fields(Some(&fields));
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].kind, TransitionFieldKind::Picker);
        assert_eq!(
            req[0].value_from_choice("10000", "Done"),
            json!({ "name": "Done" })
        );
    }

    #[test]
    fn parses_user_and_date_fields() {
        let fields = json!({
            "assignee": {
                "required": true,
                "name": "Assignee",
                "schema": { "type": "user", "system": "assignee" }
            },
            "customfield_10015": {
                "required": true,
                "name": "Start date",
                "schema": { "type": "date", "custom": "customfield_10015" }
            }
        });
        let req = parse_transition_screen_fields(Some(&fields));
        assert_eq!(req.len(), 2);
        assert!(req.iter().any(|f| f.kind == TransitionFieldKind::User));
        assert!(req.iter().any(|f| f.kind == TransitionFieldKind::Date));
    }

    #[test]
    fn boolean_field_gets_yes_no_options() {
        let fields = json!({
            "customfield_10040": {
                "required": true,
                "name": "Approved",
                "schema": { "type": "boolean" }
            }
        });
        let req = parse_transition_screen_fields(Some(&fields));
        assert_eq!(req[0].kind, TransitionFieldKind::Boolean);
        assert_eq!(req[0].options.len(), 2);
        assert_eq!(req[0].value_from_choice("true", "Yes"), json!(true));
    }

    #[test]
    fn parses_multiselect_custom_field() {
        let fields = json!({
            "customfield_10050": {
                "required": false,
                "name": "Tags",
                "schema": {
                    "type": "array",
                    "custom": "com.atlassian.jira.plugin.system.customfieldtypes:multiselect",
                    "customId": 10050
                },
                "allowedValues": [
                    { "id": "10001", "value": "Alpha" },
                    { "id": "10002", "value": "Beta" }
                ]
            }
        });
        let req = parse_transition_screen_fields(Some(&fields));
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].kind, TransitionFieldKind::MultiPicker);
        assert_eq!(req[0].options.len(), 2);
    }

    #[test]
    fn date_text_parses_iso() {
        let f = TransitionField {
            id: "d".into(),
            name: "Due".into(),
            field_type: "date".into(),
            system: String::new(),
            kind: TransitionFieldKind::Date,
            options: vec![],
        };
        assert_eq!(
            f.value_from_text("2026-05-29").unwrap(),
            json!("2026-05-29")
        );
    }
}
