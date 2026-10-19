# tick

<p align="center">
  <strong>The GitHub CLI (<code>gh</code>) equivalent for Atlassian — Jira & Confluence from your terminal and AI agents.</strong><br />
  Fast, single-binary CLI and native Model Context Protocol (MCP) server with multi-server context switching, workflow rule automation, and full JSON support.
</p>

<p align="center">
  <a href="https://github.com/aeswibon/tick/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/aeswibon/tick/ci.yml?branch=master&label=CI&style=flat-square" alt="CI Status" />
  </a>
  &nbsp;·&nbsp;
  <a href="https://github.com/aeswibon/tick/releases">
    <img src="https://img.shields.io/github/v/release/aeswibon/tick?style=flat-square" alt="Release" />
  </a>
  &nbsp;·&nbsp;
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License" />
  </a>
</p>

---

## Highlights

- **`gh`-Style Ergonomics**: Consistent, intuitive command grammar (`tick issue`, `tick project`, `tick page`, `tick space`, `tick context`).
- **Jira & Confluence in One**: Seamlessly manage Jira tickets, agile boards, sprints, Confluence spaces, and documentation pages.
- **Multi-Server Context Switching**: Work across multiple Jira Cloud, Data Center, or Jira Server instances with `tick context use <site>` or `--site <site>`.
- **Workflow Transitions with Automation Rules**: Inspect available transitions (`--list`), provide resolutions (`--resolution Done`), custom fields (`-F key=val` or `--fields '<json>'`), and transition comments (`-c`).
- **Rich Terminal Output & JSON**: Formatted tables with ANSI colors for humans, clean structured `--json` for scripts, pipes, and automation.
- **Native MCP Server (`tick mcp serve`)**: 36 comprehensive tools ready to plug into Claude Desktop, Cursor, Antigravity, and any MCP-compatible AI agent.
- **Blazing Fast**: Written in pure Rust, standalone compiled binary with zero runtime dependencies.

---

## Installation

### Homebrew (macOS & Linux)
```bash
brew tap aeswibon/tick
brew install tick
```

### Prebuilt Binaries
Download standalone binaries for macOS (Apple Silicon / Intel), Linux (x86_64), or Windows from the [Releases page](https://github.com/aeswibon/tick/releases).

### From Source (Cargo)
```bash
cargo install --path .
```

---

## Getting Started

### 1. Initialize Configuration
```bash
tick --init
```
This creates `~/.config/tick/config.toml`. Configure your email and API token:

```toml
email = "your-email@company.com"
token = "your-api-token"   # Or set TICK_TOKEN env var

[[sites]]
name = "production"
base_url = "https://company.atlassian.net"

[[sites]]
name = "staging"
base_url = "https://staging.atlassian.net"
```

### 2. Verify Connectivity
```bash
tick doctor
```

---

## Multi-Server Context Management (`tick context`)

When managing multiple Atlassian instances (e.g. personal, work, staging, production), `tick context` lets you switch your default site without passing `--site <name>` on every command:

```bash
# List all configured sites and see which is active
tick context list

# Switch default active context to production
tick context use production

# View active context details
tick context show

# Clear active context
tick context clear
```

### Precedence Hierarchy
1. CLI flag: `--site <name>`
2. Environment variable: `TICK_SITE` or `TICK_CONTEXT`
3. Stored context: `~/.config/tick/context` (set via `tick context use`)
4. Config default: `default_site = "production"` in `config.toml`
5. Single site fallback (when only one site is configured)

---

## Jira Commands Reference

### Issues (`tick issue`)

```bash
# List issues
tick issue list --project ENG --status "In Progress"
tick issue list --assignee me --limit 20
tick issue list --jql "labels = backend AND resolution is EMPTY" --json

# View issue details (summary, parent, labels, components, fix versions, subtasks, links, description, comments)
tick issue view ENG-123
tick issue view ENG-123 --comments
tick issue view ENG-123 --web    # Open in browser
tick issue view ENG-123 --json

# Create issue
tick issue create -p ENG -t Bug -s "Fix login race condition" -d "Details here" -P High -l auth,backend

# Edit issue
tick issue edit ENG-123 --summary "Updated title" --priority Medium --labels auth,frontend

# Workflow Transitions & Rule Handling
# 1. Inspect valid transitions from the current status and required screen fields
tick issue transition ENG-123 --list

# 2. Transition status with resolution, custom fields, and transition comment
tick issue transition ENG-123 --to "Done" --resolution "Fixed" -c "Resolved via PR #42"
tick issue transition ENG-123 --to "In Review" -F customfield_10010="Ready for QA"
tick issue transition ENG-123 --to "Done" --fields '{"resolution": {"name": "Fixed"}}'

# Close and reopen shortcuts
tick issue close ENG-123 --resolution "Done" -c "Shipped to production"
tick issue reopen ENG-123 -c "Reopening for regression verification"

# Assign issue
tick issue assign ENG-123 --me
tick issue assign ENG-123 --to 60b123456789abc
tick issue assign ENG-123 --unassign

# Comments
tick issue comment list ENG-123
tick issue comment add ENG-123 -b "PR is ready for review."
tick issue comment delete ENG-123 <comment_id> --yes

# Worklogs
tick issue worklog list ENG-123
tick issue worklog add ENG-123 "2h 30m"
tick issue worklog delete ENG-123 <worklog_id> --yes

# Issue Links & Watchers
tick issue link ENG-123 ENG-456 --type "Blocks"
tick issue watch ENG-123 --me
tick issue unwatch ENG-123 --me

# Delete issue
tick issue delete ENG-123 --yes
```

### Projects (`tick project`)

```bash
# List projects
tick project list

# View project details
tick project view ENG

# List issue types and workflow statuses configured for a project
tick project statuses ENG

# List project components
tick project components ENG

# List project release versions
tick project versions ENG
```

### Agile Boards & Sprints (`tick board`, `tick sprint`)

```bash
# List agile boards
tick board list

# View issues on a board or backlog
tick board issues 42
tick board backlog 42

# Sprints
tick sprint list 42          # List active and planned sprints
tick sprint issues 105       # List issues in a sprint
tick sprint move ENG-123 105 # Move issue to sprint
```

### Fields & Users (`tick field`, `tick user`)

```bash
# Field discovery
tick field list
tick field list --custom-only

# Users
tick user me
tick user search "Alice"
tick user assignable ENG
```

### Bulk Operations (`tick bulk`)

```bash
tick bulk transition Done --keys ENG-1,ENG-2,ENG-3
tick bulk assign --to me --keys ENG-1,ENG-2
tick bulk comment "Batch triage complete" --keys ENG-1,ENG-2
```

---

## Confluence Commands Reference

### Spaces (`tick space`)

```bash
# List spaces
tick space list --limit 20
tick space list --json

# View space metadata
tick space view ENG
tick space view ENG --web    # Open in browser
```

### Pages (`tick page`)

```bash
# List pages in a space
tick page list <space_id>

# View page content (converts Confluence storage XHTML to readable markdown)
tick page view <page_id>
tick page view <page_id> --raw   # Show raw XHTML
tick page view <page_id> --web   # Open in browser
tick page view <page_id> --json

# Create page
tick page create --space <space_id> --title "API Architecture" --body "Overview of architecture..."
cat document.md | tick page create --space <space_id> --title "RFC: New Auth"

# Edit page
tick page edit <page_id> --title "Updated Title" --body "New content..."

# Comments & Version History
tick page comment <page_id> -b "Looks good to me!"
tick page history <page_id>
tick page attachments <page_id>

# Delete page
tick page delete <page_id> --yes
```

---

## Raw REST API & Search

### REST API Client (`tick api`)
Execute arbitrary authenticated requests against Jira or Confluence:

```bash
# GET current user
tick api /rest/api/3/myself

# POST with fields
tick api /rest/api/3/issue -X POST --input issue.json

# Confluence API v2
tick api /wiki/api/v2/spaces -X GET
```

### Global Search (`tick search`)

```bash
# Search Jira issues via JQL or free-text
tick search issues "login bug"
tick search issues --jql "project = ENG AND text ~ 'authentication'"

# Search Confluence pages
tick search pages "deployment runbook"
```

---

## Model Context Protocol (MCP) Server

`tick` includes a high-performance, native stdio MCP server exposing **36 tools** for AI agents.

### Start MCP Server
```bash
tick mcp serve
```

### Claude Desktop Configuration
Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "tick": {
      "command": "tick",
      "args": ["mcp", "serve"],
      "env": {
        "TICK_TOKEN": "your-api-token"
      }
    }
  }
}
```

### Available MCP Tools

| Category | Tools | Description |
|----------|-------|-------------|
| **Jira Issues** | `jira_search`, `jira_get_issue`, `jira_create_issue`, `jira_edit_issue`, `jira_delete_issue`, `jira_assign_issue` | Full issue lifecycle and JQL search |
| **Workflow Rules** | `jira_list_transitions`, `jira_transition_issue` | Inspect transition screens, apply resolutions, custom fields & comments |
| **Comments & Worklogs** | `jira_add_comment`, `jira_list_comments`, `jira_delete_comment`, `jira_add_worklog`, `jira_list_worklogs`, `jira_delete_worklog` | Time tracking and issue conversations |
| **Issue Links** | `jira_link_issues` | Create relations (Blocks, Relates, Clones, etc.) |
| **Projects & Boards** | `jira_list_projects`, `jira_get_project`, `jira_list_project_components`, `jira_list_project_versions`, `jira_list_boards`, `jira_list_sprints` | Agile hierarchy, components, and releases |
| **Users** | `jira_list_users`, `jira_get_current_user` | User directory and assignees |
| **Confluence Spaces** | `confluence_list_spaces`, `confluence_get_space` | Spaces discovery and details |
| **Confluence Pages** | `confluence_list_pages`, `confluence_search`, `confluence_get_page`, `confluence_create_page`, `confluence_update_page`, `confluence_delete_page` | Documentation authoring and search |
| **Confluence Content** | `confluence_list_comments`, `confluence_add_comment`, `confluence_list_page_versions`, `confluence_list_attachments` | Page revisions, comments, attachments |
| **Universal API** | `atlassian_raw_api` | Raw authenticated Jira & Confluence API endpoints |

---

## Configuration Reference

Default config path: `~/.config/tick/config.toml` (or `$XDG_CONFIG_HOME/tick/config.toml`).

```toml
# Authentication: basic token (default) or oauth
auth = "token"
email = "developer@company.com"
token = "your-jira-api-token"  # Or TICK_TOKEN env var / ~/.config/tick/token

# Default active site (can be overridden with 'tick context use' or --site)
default_site = "production"

# Configured sites
[[sites]]
name = "production"
base_url = "https://company.atlassian.net"
board_id = 42

[[sites]]
name = "staging"
base_url = "https://staging.atlassian.net"
```

---

## License

MIT © [tick contributors](https://github.com/aeswibon/tick)
