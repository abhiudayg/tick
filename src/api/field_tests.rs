use super::*;

#[tokio::test]
async fn find_issue_browse_url_probes_sites_in_order() {
    let first = wiremock::MockServer::start().await;
    let second = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/issue/FOO-1"))
        .respond_with(wiremock::ResponseTemplate::new(404))
        .mount(&first)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/issue/FOO-1"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "key": "FOO-1" })),
        )
        .mount(&second)
        .await;

    let sites = vec![
        crate::config::Site {
            name: "a".into(),
            base_url: first.uri(),
            ..Default::default()
        },
        crate::config::Site {
            name: "b".into(),
            base_url: second.uri(),
            ..Default::default()
        },
    ];
    let client = JiraClient::new("u@example.com", "token", false);
    let url = client.find_issue_browse_url(&sites, "FOO-1").await.unwrap();
    assert_eq!(url, format!("{}/browse/FOO-1", second.uri()));
}

#[tokio::test]
async fn update_summary_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "fields": { "summary": "New title" } }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .update_summary(&server.uri(), "DEMO-1", "New title")
        .await
        .unwrap();
}

#[tokio::test]
async fn update_priority_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "fields": { "priority": { "name": "High" } }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .update_priority(&server.uri(), "DEMO-1", "High")
        .await
        .unwrap();
}

#[tokio::test]
async fn update_description_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .update_description(&server.uri(), "DEMO-1", "# Hello\n\nWorld", &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn search_assignable_users_parses_response() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/rest/api/3/user/assignable/search",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "accountId": "acc-1", "displayName": "Alice" },
                { "accountId": "acc-2", "displayName": "Bob" }
            ])),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let catalog = client
        .ensure_assignable_users(&server.uri(), "DEMO-1", "", false)
        .await
        .unwrap();
    assert_eq!(catalog.len(), 2);

    let users = client
        .filter_assignable_users(&server.uri(), "DEMO-1", "al")
        .await
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0], ("acc-1".into(), "Alice".into()));

    // Refresh merges (same response still dedupes to 2).
    let merged = client
        .ensure_assignable_users(&server.uri(), "DEMO-1", "", true)
        .await
        .unwrap();
    assert_eq!(merged.len(), 2);

    // Second load uses cache (no extra HTTP).
    let again = client
        .ensure_assignable_users(&server.uri(), "DEMO-1", "", false)
        .await
        .unwrap();
    assert_eq!(again.len(), 2);
}

#[tokio::test]
async fn add_comment_with_pdf_attachment_uploads_and_links() {
    let server = wiremock::MockServer::start().await;
    let dir = std::env::temp_dir().join(format!("tick-att-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("note.txt");
    std::fs::write(&path, b"hello").unwrap();

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/attachments",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "content": "https://jira.example/attachment/1"
            }])),
        )
        .mount(&server)
        .await;

    let mut expected = crate::api::markdown::to_adf("see file", &[]);
    expected["content"].as_array_mut().unwrap().push(
        crate::api::attachments::attachment_link_paragraph(
            &crate::api::attachments::UploadedAttachment {
                filename: "note.txt".into(),
                content_url: "https://jira.example/attachment/1".into(),
            },
        ),
    );

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1/comment"))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "body": expected }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .add_comment_with_attachments(
            &server.uri(),
            "DEMO-1",
            "see file",
            &[],
            std::slice::from_ref(&path),
        )
        .await
        .unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn add_comment_with_mention_sends_adf() {
    let server = wiremock::MockServer::start().await;
    let mentions = vec![("@Alice".into(), "acc-1".into())];
    let body = crate::api::adf::comment_body_with_mentions("hi @Alice", &mentions);
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1/comment"))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "body": body }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .add_comment(&server.uri(), "DEMO-1", "hi @Alice", &mentions)
        .await
        .unwrap();
}

#[tokio::test]
async fn watch_issue_posts_watcher() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/myself"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "accountId": "me-1" })),
        )
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/watchers",
        ))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "accountId": "me-1" }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client.watch_issue(&server.uri(), "DEMO-1").await.unwrap();
}

#[tokio::test]
async fn update_due_date_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "fields": { "duedate": "2026-12-31" }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let d = chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    client
        .update_due_date(&server.uri(), "DEMO-1", Some(d))
        .await
        .unwrap();
}

#[tokio::test]
async fn update_labels_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "fields": { "labels": ["backend", "urgent"] }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .update_labels(
            &server.uri(),
            "DEMO-1",
            &["backend".into(), "urgent".into()],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn list_priorities_parses_response() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/priority"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": "1", "name": "High" },
                { "id": "2", "name": "Low" }
            ])),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let list = client.list_priorities(&server.uri()).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].1, "High");
}

#[tokio::test]
async fn assign_to_account_sends_put() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "fields": { "assignee": { "accountId": "acc-99" } }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .assign_to_account(&server.uri(), "DEMO-1", "acc-99")
        .await
        .unwrap();
}

#[tokio::test]
async fn unassign_clears_assignee() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "fields": { "assignee": null }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client.unassign(&server.uri(), "DEMO-1").await.unwrap();
}

#[tokio::test]
async fn get_workflow_transitions_parses_target_status() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transitions": [
                    { "id": "21", "name": "Done", "to": { "name": "Done" } },
                    { "id": "31", "name": "Start Progress", "to": { "name": "In Progress" } }
                ]
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let options = client
        .get_workflow_transitions(&server.uri(), "DEMO-1")
        .await
        .unwrap();
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].to_status, "Done");
    assert_eq!(options[1].label(), "Start Progress → In Progress");
}

#[tokio::test]
async fn transition_issue_posts_transition_id() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "transition": { "id": "21" } }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let tr = types::WorkflowTransition {
        id: "21".into(),
        name: "Done".into(),
        to_status: "Done".into(),
        required_fields: vec![],
    };
    client
        .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_workflow_transitions_parses_required_fields() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transitions": [{
                    "id": "21",
                    "name": "Done",
                    "to": { "name": "Done" },
                    "fields": {
                        "resolution": {
                            "required": true,
                            "name": "Resolution",
                            "schema": { "type": "resolution" },
                            "allowedValues": [
                                { "id": "10000", "name": "Done" }
                            ]
                        }
                    }
                }]
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let options = client
        .get_workflow_transitions(&server.uri(), "DEMO-1")
        .await
        .unwrap();
    assert_eq!(options[0].required_fields.len(), 1);
    assert_eq!(options[0].required_fields[0].id, "resolution");
}

#[tokio::test]
async fn transition_issue_posts_required_fields() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "transition": { "id": "21" },
            "fields": { "resolution": { "name": "Done" } }
        })))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let tr = types::WorkflowTransition {
        id: "21".into(),
        name: "Done".into(),
        to_status: "Done".into(),
        required_fields: vec![],
    };
    let mut fields = HashMap::new();
    fields.insert("resolution".into(), serde_json::json!({ "name": "Done" }));
    client
        .transition_issue(&server.uri(), "DEMO-1", &tr, &fields)
        .await
        .unwrap();
}

#[tokio::test]
async fn workflow_transition_reports_validation_error() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "errorMessages": ["Resolution is required"],
                "errors": {}
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let tr = types::WorkflowTransition {
        id: "21".into(),
        name: "Done".into(),
        to_status: "Done".into(),
        required_fields: vec![],
    };
    let err = client
        .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
        .await
        .unwrap_err();
    let errs =
        jira_error::field_errors(r#"{"errorMessages":["Resolution is required"],"errors":{}}"#);
    assert_eq!(errs.len(), 1);
    assert!(err.message.contains("Resolution is required"));
}

#[tokio::test]
async fn workflow_transition_field_errors_parsed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/rest/api/3/issue/DEMO-1/transitions",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "errorMessages": [],
                "errors": { "resolution": "Resolution is required" }
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let tr = types::WorkflowTransition {
        id: "21".into(),
        name: "Done".into(),
        to_status: "Done".into(),
        required_fields: vec![],
    };
    let err = client
        .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
        .await
        .unwrap_err();
    assert_eq!(err.field_errors.len(), 1);
    assert_eq!(err.field_errors[0].0, "resolution");
}

#[tokio::test]
async fn add_worklog_sends_time_spent() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1/worklog"))
        .and(wiremock::matchers::body_json(
            serde_json::json!({ "timeSpent": "30m" }),
        ))
        .respond_with(wiremock::ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    client
        .add_worklog(&server.uri(), "DEMO-1", "30m")
        .await
        .unwrap();
}

#[tokio::test]
async fn issue_exists_on_success_and_missing() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/issue/FOUND-1"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "key": "FOUND-1" })),
        )
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/issue/MISSING-1"))
        .respond_with(wiremock::ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    assert!(client.issue_exists(&server.uri(), "FOUND-1").await);
    assert!(!client.issue_exists(&server.uri(), "MISSING-1").await);
}

#[tokio::test]
async fn current_user_account_id_reads_myself() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/myself"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "accountId": "me-42" })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let id = client.current_user_account_id(&server.uri()).await.unwrap();
    assert_eq!(id, "me-42");
    let cached = client.current_user_account_id(&server.uri()).await.unwrap();
    assert_eq!(cached, "me-42");
}

#[tokio::test]
async fn create_issue_posts_fields_and_returns_key() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue"))
        .respond_with(
            wiremock::ResponseTemplate::new(201)
                .set_body_json(serde_json::json!({ "key": "DEMO-99", "id": "10099" })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let fields = serde_json::json!({
        "project": { "key": "DEMO" },
        "issuetype": { "name": "Task" },
        "summary": "New from tick"
    });
    let key = client
        .create_issue(&server.uri(), &fields)
        .await
        .expect("create");
    assert_eq!(key, "DEMO-99");
}

#[tokio::test]
async fn create_issue_reports_field_errors() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue"))
        .respond_with(
            wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "errorMessages": [],
                "errors": { "components": "Components is required." }
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("u@example.com", "token", false);
    let fields = serde_json::json!({
        "project": { "key": "DEMO" },
        "issuetype": { "name": "Task" },
        "summary": "X"
    });
    let err = client
        .create_issue(&server.uri(), &fields)
        .await
        .expect_err("validation");
    assert!(!err.field_errors.is_empty());
    assert_eq!(err.field_errors[0].0, "components");
}
