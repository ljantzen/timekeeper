use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "tmkpr",
    about = "A natural-language time tracking CLI",
    version
)]
pub struct Cli {
    /// Path to the SQLite database (overrides config)
    #[arg(long, env = "TMKPR_DB", global = true)]
    pub db: Option<std::path::PathBuf>,

    /// Output format
    #[arg(
        long,
        short = 'f',
        global = true,
        default_value = "table",
        value_parser = ["table", "json", "csv"]
    )]
    pub format: String,

    /// Disable color output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start tracking time
    #[command(alias = "track")]
    Start(StartArgs),

    /// Stop the current tracking session
    #[command(alias = "finish")]
    Stop(StopArgs),

    /// Add a completed time entry directly
    #[command(alias = "record")]
    Log(LogArgs),

    /// Show the currently active entry
    Status,

    /// List time entries
    List(ListArgs),

    /// Show a summarized report grouped by project/task
    Report(ReportArgs),

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCommands),

    /// Edit a time entry
    Edit(EditArgs),

    /// Delete a time entry
    Delete(DeleteArgs),

    /// Generate shell completion scripts
    Completion(CompletionArgs),
}

// ── Start ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct StartArgs {
    /// Project name
    #[arg(short, long)]
    pub project: Option<String>,

    /// Task name (requires --project)
    #[arg(short, long)]
    pub task: Option<String>,

    /// Note / description
    #[arg(short, long)]
    pub note: Option<String>,

    /// Start time — natural language or ISO 8601 (e.g. "2 hours ago", "9am")
    #[arg(short, long)]
    pub start: Option<String>,

    /// Tags (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,
}

// ── Stop ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct StopArgs {
    /// Finish time — natural language or ISO 8601
    #[arg(short, long)]
    pub end: Option<String>,
}

// ── Log ───────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct LogArgs {
    /// Start time — natural language or ISO 8601 (omit to use last entry's end time)
    #[arg(short, long)]
    pub start: Option<String>,

    /// End time — natural language or ISO 8601 (defaults to now)
    #[arg(short, long)]
    pub end: Option<String>,

    /// Project name or numeric ID
    #[arg(short, long)]
    pub project: Option<String>,

    /// Task name or numeric ID (requires --project)
    #[arg(short, long)]
    pub task: Option<String>,

    /// Note / description
    #[arg(short, long)]
    pub note: Option<String>,

    /// Tags (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,
}

// ── List ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct ListArgs {
    /// Filter by project name
    #[arg(short, long)]
    pub project: Option<String>,

    /// Filter by task name
    #[arg(short, long)]
    pub task: Option<String>,

    /// Start of date range (natural language or ISO 8601)
    #[arg(long)]
    pub from: Option<String>,

    /// End of date range (natural language or ISO 8601)
    #[arg(long)]
    pub until: Option<String>,

    /// Maximum number of entries to show
    #[arg(short, long)]
    pub limit: Option<u32>,

    /// Include the currently active entry
    #[arg(long, default_value_t = true)]
    pub active: bool,

    /// Filter by tag (can be specified multiple times)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Show untracked gaps instead of entries
    #[arg(long)]
    pub gaps: bool,
}

// ── Report ────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct ReportArgs {
    /// Start of date range
    #[arg(long)]
    pub from: Option<String>,

    /// End of date range
    #[arg(long)]
    pub until: Option<String>,

    /// Limit report to a single project
    #[arg(short, long)]
    pub project: Option<String>,
}

// ── Project subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Add a new project
    Add(ProjectAddArgs),
    /// List all projects
    List(ProjectListArgs),
    /// Edit a project
    Edit(ProjectEditArgs),
    /// Delete (archive) a project
    Delete(ProjectDeleteArgs),
}

#[derive(Args)]
pub struct ProjectAddArgs {
    pub name: String,
    #[arg(short, long)]
    pub description: Option<String>,
    /// Hex color code (e.g. #ff5733) for TUI display
    #[arg(long)]
    pub color: Option<String>,
}

#[derive(Args)]
pub struct ProjectListArgs {
    /// Include archived projects
    #[arg(long)]
    pub archived: bool,
}

#[derive(Args)]
pub struct ProjectEditArgs {
    /// Project name or numeric ID
    pub project: String,
    /// New name
    #[arg(long)]
    pub name: Option<String>,
    /// New description (use "-" to clear)
    #[arg(short, long)]
    pub description: Option<String>,
    /// New hex color (use "-" to clear)
    #[arg(long)]
    pub color: Option<String>,
}

#[derive(Args)]
pub struct ProjectDeleteArgs {
    pub name: String,
    /// Permanently delete instead of archiving
    #[arg(long)]
    pub hard: bool,
}

// ── Task subcommands ──────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Add a new task
    Add(TaskAddArgs),
    /// List tasks for a project
    List(TaskListArgs),
    /// Edit a task
    Edit(TaskEditArgs),
    /// Delete (archive) a task
    Delete(TaskDeleteArgs),
}

#[derive(Args)]
pub struct TaskAddArgs {
    pub name: String,
    #[arg(short, long)]
    pub project: String,
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct TaskListArgs {
    #[arg(short, long)]
    pub project: String,
    /// Include archived tasks
    #[arg(long)]
    pub archived: bool,
}

#[derive(Args)]
pub struct TaskEditArgs {
    /// Task name or numeric ID
    pub task: String,
    /// Project name or numeric ID (current project, used to locate the task)
    #[arg(short, long)]
    pub project: String,
    /// Move task to a different project (name or numeric ID)
    #[arg(long)]
    pub move_to: Option<String>,
    /// New name
    #[arg(long)]
    pub name: Option<String>,
    /// New description (use "-" to clear)
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct TaskDeleteArgs {
    pub name: String,
    #[arg(short, long)]
    pub project: String,
    /// Permanently delete instead of archiving
    #[arg(long)]
    pub hard: bool,
}

// ── Edit ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct EditArgs {
    /// Entry ID or UUID prefix (at least 8 chars)
    pub id: String,

    #[arg(short, long)]
    pub project: Option<String>,

    #[arg(short, long)]
    pub task: Option<String>,

    #[arg(short, long)]
    pub note: Option<String>,

    /// New start time
    #[arg(long)]
    pub start: Option<String>,

    /// New end time
    #[arg(long)]
    pub end: Option<String>,

    /// Replace all tags (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
}

// ── Delete ────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct DeleteArgs {
    /// Entry ID or UUID prefix
    pub id: String,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

// ── Completion ────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct CompletionArgs {
    pub shell: Shell,
}
