//! `tick issue` commands modeled after `gh issue`.

pub mod extra;
pub mod list;
pub mod manage;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct IssueArgs {
    #[command(subcommand)]
    pub command: IssueCommand,
}

#[derive(Subcommand, Debug)]
pub enum IssueCommand {
    /// List issues matching query or assigned to you
    #[command(name = "list", alias = "ls")]
    List(list::IssueListArgs),

    /// View issue details, comments, and links
    #[command(name = "view")]
    View(list::IssueViewArgs),

    /// Create a new issue
    #[command(name = "create", alias = "new")]
    Create(manage::IssueCreateArgs),

    /// Edit an existing issue
    #[command(name = "edit")]
    Edit(manage::IssueEditArgs),

    /// Close an issue
    #[command(name = "close")]
    Close(manage::IssueCloseArgs),

    /// Reopen an issue
    #[command(name = "reopen")]
    Reopen(manage::IssueReopenArgs),

    /// Transition issue workflow status
    #[command(name = "transition", alias = "move")]
    Transition(manage::IssueTransitionArgs),

    /// Assign an issue to a user or clear assignee
    #[command(name = "assign")]
    Assign(manage::IssueAssignArgs),

    /// Delete an issue
    #[command(name = "delete", alias = "rm")]
    Delete(manage::IssueDeleteArgs),

    /// Add, list, or delete comments on an issue
    #[command(name = "comment")]
    Comment(extra::CommentArgs),

    /// Log, list, or delete worklog entries on an issue
    #[command(name = "worklog")]
    Worklog(extra::WorklogArgs),

    /// Link issues together or list relations
    #[command(name = "link")]
    Link(extra::LinkArgs),

    /// Add current user as watcher
    #[command(name = "watch")]
    Watch(extra::WatchArgs),

    /// Remove current user from watchers
    #[command(name = "unwatch")]
    Unwatch(extra::WatchArgs),
}

pub async fn run(cmd: IssueCommand) -> Result<(), String> {
    match cmd {
        IssueCommand::List(args) => list::run_list(args).await,
        IssueCommand::View(args) => list::run_view(args).await,
        IssueCommand::Create(args) => manage::run_create(args).await,
        IssueCommand::Edit(args) => manage::run_edit(args).await,
        IssueCommand::Close(args) => manage::run_close(args).await,
        IssueCommand::Reopen(args) => manage::run_reopen(args).await,
        IssueCommand::Transition(args) => manage::run_transition(args).await,
        IssueCommand::Assign(args) => manage::run_assign(args).await,
        IssueCommand::Delete(args) => manage::run_delete(args).await,
        IssueCommand::Comment(args) => extra::run_comment(args).await,
        IssueCommand::Worklog(args) => extra::run_worklog(args).await,
        IssueCommand::Link(args) => extra::run_link(args).await,
        IssueCommand::Watch(args) => extra::run_watch(args, false).await,
        IssueCommand::Unwatch(args) => extra::run_watch(args, true).await,
    }
}
