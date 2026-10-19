use super::*;
use crate::api::types::{
    BulkFetchIssue, JiraFields, JiraNamed, JiraProject, JiraStatus, JiraStatusCategory, Ticket,
};
use crate::config::{Config, Site};
use std::collections::HashMap;

fn test_config(base_url: &str) -> Config {
    Config {
        email: "user@example.com".into(),
        token: "token".into(),
        sites: vec![Site {
            name: "test".into(),
            base_url: base_url.into(),
            ..Default::default()
        }],
        default_site: None,
        columns: None,
        max_results: 50,
        page_size: 10,
        theme: "default".into(),
        views: Default::default(),
        notify_on_refresh: false,
        auth: Default::default(),
        oauth: Default::default(),
        create: Default::default(),
        hooks: Default::default(),
        detail: Default::default(),
        notice_secs: 5,
        view_jql: Config::build_view_jql(&Default::default()),
    }
}

#[tokio::test]
async fn fetch_issue_detail_loads_description_and_comments() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
        .and(wiremock::matchers::query_param(
            "fields",
            "description,comment",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "fields": {
                    "description": { "type": "doc", "version": 1, "content": [
                        { "type": "paragraph", "content": [
                            { "type": "text", "text": "Hello" }
                        ]}
                    ]},
                    "comment": { "comments": [] }
                }
            })),
        )
        .mount(&server)
        .await;

    let client = JiraClient::new("user@example.com", "token", false);
    let fields = client
        .fetch_issue_detail(&server.uri(), "DEMO-1")
        .await
        .expect("detail fetch");
    let mut ticket = Ticket::from_bulk_fetch(
        BulkFetchIssue {
            key: "DEMO-1".into(),
            fields: JiraFields {
                issue_type: JiraNamed {
                    name: "Task".into(),
                },
                status: JiraStatus {
                    name: "Open".into(),
                    status_category: JiraStatusCategory {
                        key: "new".into(),
                        color_name: "blue".into(),
                    },
                },
                priority: None,
                assignee: None,
                reporter: None,
                duedate: None,
                created: "2026-01-01T00:00:00.000+0000".into(),
                project: JiraProject { key: "DEMO".into() },
                summary: "S".into(),
                description: None,
                comment: None,
                parent: None,
                labels: None,
                custom: HashMap::new(),
            },
        },
        "test",
        &server.uri(),
        None,
        &[],
        false,
    );
    assert!(!ticket.detail_loaded);
    ticket.apply_detail_fields(&fields);
    assert!(ticket.detail_loaded);
    assert!(ticket
        .description
        .as_ref()
        .is_some_and(|d| d.contains("Hello")));
}

#[tokio::test]
async fn search_jql_paginates_and_respects_max_results() {
    let server = wiremock::MockServer::start().await;

    let page1_issues: Vec<_> = (1..=100)
        .map(|n| serde_json::json!({ "id": format!("id{n:03}") }))
        .collect();

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/search/jql"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "jql": "project = DEMO",
            "maxResults": 100
        })))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": page1_issues,
                "nextPageToken": "page-2",
                "isLast": false
            })),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let page2_issues: Vec<_> = (101..=150)
        .map(|n| serde_json::json!({ "id": format!("id{n:03}") }))
        .collect();

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/search/jql"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "jql": "project = DEMO",
            "maxResults": 20,
            "nextPageToken": "page-2"
        })))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": page2_issues,
                "isLast": true
            })),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let client = JiraClient::new("user@example.com", "token", false);
    let ids = client
        .search_jql(&server.uri(), "project = DEMO", 120, &mut None)
        .await
        .expect("search");

    assert_eq!(ids.len(), 120);
    assert_eq!(ids[0], "id001");
    assert_eq!(ids[119], "id120");
}

#[tokio::test]
async fn fetch_all_maps_jql_and_bulk_fetch() {
    let server = wiremock::MockServer::start().await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/search/jql"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [{ "id": "10001" }]
            })),
        )
        .mount(&server)
        .await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue/bulkfetch"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [{
                    "key": "DEMO-1",
                    "fields": {
                        "issuetype": { "name": "Task" },
                        "status": {
                            "name": "To Do",
                            "statusCategory": { "key": "new", "colorName": "blue-gray" }
                        },
                        "priority": { "name": "Medium" },
                        "assignee": { "displayName": "Alice" },
                        "reporter": { "displayName": "Bob" },
                        "created": "2026-01-01T00:00:00.000+0000",
                        "project": { "key": "DEMO" },
                        "summary": "Hello",
                        "labels": ["bug", "ui"]
                    }
                }]
            })),
        )
        .mount(&server)
        .await;

    let config = test_config(&server.uri());
    let client = JiraClient::new("user@example.com", "token", false);
    let jql = config.jql_for(crate::view_mode::ViewMode::MyIssues);
    let (tickets, errors) = fetch_all(&client, &config, jql, None, &[], &mut None).await;

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].key, "DEMO-1");
    assert_eq!(tickets[0].site, "test");
    assert_eq!(tickets[0].summary, "Hello");
    assert_eq!(tickets[0].labels, vec!["bug", "ui"]);
}

#[tokio::test]
async fn fetch_all_keeps_successful_sites_when_another_site_fails() {
    let failing = wiremock::MockServer::start().await;
    let passing = wiremock::MockServer::start().await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/search/jql"))
        .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&failing)
        .await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/search/jql"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [{ "id": "20001" }]
            })),
        )
        .mount(&passing)
        .await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/rest/api/3/issue/bulkfetch"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [{
                    "key": "PASS-1",
                    "fields": {
                        "issuetype": { "name": "Bug" },
                        "status": {
                            "name": "In Progress",
                            "statusCategory": { "key": "indeterminate", "colorName": "yellow" }
                        },
                        "priority": null,
                        "assignee": null,
                        "reporter": null,
                        "duedate": null,
                        "created": "2026-01-01T00:00:00.000+0000",
                        "project": { "key": "PASS" },
                        "summary": "Still fetched",
                        "labels": null
                    }
                }]
            })),
        )
        .mount(&passing)
        .await;

    let mut config = test_config(&failing.uri());
    config.sites[0].name = "broken".into();
    config.sites.push(Site {
        name: "healthy".into(),
        base_url: passing.uri(),
        ..Default::default()
    });

    let client = JiraClient::new("user@example.com", "token", false);
    let jql = config.jql_for(crate::view_mode::ViewMode::MyIssues);
    let (tickets, errors) = fetch_all(&client, &config, jql, None, &[], &mut None).await;

    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].key, "PASS-1");
    assert_eq!(tickets[0].site, "healthy");
    assert_eq!(tickets[0].priority, "");
    assert_eq!(tickets[0].assignee, "");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("broken:"));
    assert!(errors[0].contains("401"));
}
