//! ViewMode enum for predefined Jira views in config.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewMode {
    MyIssues,
    Mentions,
    Watching,
    Updated,
    Sprint,
    ClosedSearch,
}

impl ViewMode {
    pub fn all() -> [ViewMode; 6] {
        [
            ViewMode::MyIssues,
            ViewMode::Mentions,
            ViewMode::Watching,
            ViewMode::Updated,
            ViewMode::Sprint,
            ViewMode::ClosedSearch,
        ]
    }

    pub fn default_jql(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => {
                "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Mentions => "text ~ currentUser() ORDER BY updated DESC",
            ViewMode::Watching => {
                "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Updated => "updated >= -1w ORDER BY updated DESC",
            ViewMode::Sprint => {
                "sprint in openSprints() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::ClosedSearch => "statusCategory = Done ORDER BY updated DESC",
        }
    }

    pub fn closed_search_base(ever_assigned: bool) -> &'static str {
        if ever_assigned {
            "assignee was currentUser() AND statusCategory = Done ORDER BY updated DESC"
        } else {
            "statusCategory = Done ORDER BY updated DESC"
        }
    }
}

pub fn build_closed_search_jql(base: &str, query: &str) -> String {
    let q = query.trim();
    if q.is_empty() {
        return base.to_string();
    }
    format!("text ~ \"{q}\" AND ({base})")
}
