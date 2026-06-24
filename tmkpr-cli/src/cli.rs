use clap::{Args, Parser, Subcommand};
use clap_complete::ArgValueCompleter;
use clap_complete::Shell;

use crate::completers::{complete_projects, complete_tasks};

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
        value_parser = ["table", "json", "csv", "markdown"]
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
    #[command(aliases = ["track", "s"])]
    Start(StartArgs),

    /// Stop the current tracking session
    #[command(aliases = ["finish", "x"])]
    Stop(StopArgs),

    /// Add a completed time entry directly
    #[command(aliases = ["record", "l"])]
    Log(LogArgs),

    /// Show the currently active entry
    #[command(alias = "st")]
    Status,

    /// List time entries
    #[command(alias = "ls")]
    List(ListArgs),

    /// Show a summarized report grouped by project/task
    #[command(alias = "r")]
    Report(ReportArgs),

    /// Manage projects
    #[command(subcommand, alias = "p")]
    Project(ProjectCommands),

    /// Manage tasks
    #[command(subcommand, alias = "t")]
    Task(TaskCommands),

    /// Edit a time entry
    #[command(alias = "e")]
    Edit(EditArgs),

    /// Delete a time entry
    #[command(aliases = ["d", "rm"])]
    Delete(DeleteArgs),

    /// Merge an entry with the next entry sharing its project and task
    #[command(alias = "m")]
    Merge(MergeArgs),

    /// Extend an entry's boundaries to fill time gaps with adjacent entries
    #[command(alias = "fg")]
    FillGap(FillGapArgs),

    /// Manage point-in-time events
    #[command(subcommand, alias = "ev")]
    Event(EventCommands),

    /// Manage comments on time entries
    #[command(subcommand, alias = "c")]
    Comment(CommentCommands),

    /// Generate shell completion scripts
    Completion(CompletionArgs),

    /// Import projects, tasks, and time entries from a CSV file
    Import(ImportArgs),

    /// Export time entries to a CSV file
    Export(ExportArgs),

    /// Launch the tmkpr-ui terminal dashboard
    #[command(alias = "u")]
    Ui(LaunchArgs),

    /// Launch the tmkpr-pomodoro timer
    #[command(aliases = ["pomo", "p25"])]
    Pomodoro(LaunchArgs),
}

// ── Start ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct StartArgs {
    /// Project name
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Task name (requires --project)
    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
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

    /// Stop active entry without prompting (handoff at --start time or now)
    #[arg(short = 'f', long)]
    pub force: bool,
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
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Task name or numeric ID (requires --project)
    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
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
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Filter by task name
    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
    pub task: Option<String>,

    /// Start of date range (natural language or ISO 8601) [default: today]
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

    /// Hide gaps shorter than this many minutes (only used with --gaps)
    #[arg(long, value_name = "MINUTES", default_value_t = 0)]
    pub min_gap: u32,
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
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Per-day project summary for an ISO week number (default: current week)
    #[arg(long, value_name = "WEEK", num_args = 0..=1, default_missing_value = "current")]
    pub week: Option<String>,

    /// Year to use with --week / --wweek (defaults to current year)
    #[arg(long, value_name = "YEAR")]
    pub year: Option<i32>,

    /// Working-week report (Mon–Fri only); use alone for the current week or combine with --week
    #[arg(long)]
    pub wweek: bool,
}

// ── Project subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Add a new project
    #[command(alias = "a")]
    Add(ProjectAddArgs),
    /// List all projects
    #[command(alias = "ls")]
    List(ProjectListArgs),
    /// Edit a project
    #[command(alias = "e")]
    Edit(ProjectEditArgs),
    /// Delete (archive) a project
    #[command(aliases = ["d", "rm"])]
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
    #[arg(add = ArgValueCompleter::new(complete_projects))]
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
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    pub name: String,
    /// Permanently delete instead of archiving
    #[arg(long)]
    pub hard: bool,
}

// ── Task subcommands ──────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Add a new task
    #[command(alias = "a")]
    Add(TaskAddArgs),
    /// List tasks for a project
    #[command(alias = "ls")]
    List(TaskListArgs),
    /// Edit a task
    #[command(alias = "e")]
    Edit(TaskEditArgs),
    /// Delete (archive) a task
    #[command(aliases = ["d", "rm"])]
    Delete(TaskDeleteArgs),
    /// Mark a task as completed
    Done(TaskDoneArgs),
    /// Reactivate a completed task
    Reactivate(TaskReactivateArgs),
}

#[derive(Args)]
pub struct TaskAddArgs {
    pub name: String,
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: String,
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct TaskListArgs {
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: String,
    /// Include archived tasks
    #[arg(long)]
    pub archived: bool,
}

#[derive(Args)]
pub struct TaskEditArgs {
    /// Task name or numeric ID
    #[arg(add = ArgValueCompleter::new(complete_tasks))]
    pub task: String,
    /// Project name or numeric ID (current project, used to locate the task)
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
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
    #[arg(add = ArgValueCompleter::new(complete_tasks))]
    pub name: String,
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: String,
    /// Permanently delete instead of archiving
    #[arg(long)]
    pub hard: bool,
}

#[derive(Args)]
pub struct TaskDoneArgs {
    /// Task name or numeric ID
    #[arg(add = ArgValueCompleter::new(complete_tasks))]
    pub task: String,
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: String,
}

#[derive(Args)]
pub struct TaskReactivateArgs {
    /// Task name or numeric ID
    #[arg(add = ArgValueCompleter::new(complete_tasks))]
    pub task: String,
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: String,
}

// ── Edit ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct EditArgs {
    /// Entry ID or UUID prefix (at least 8 chars)
    pub id: String,

    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
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

// ── Merge ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct MergeArgs {
    /// Entry ID or UUID prefix
    pub id: String,
}

// ── Fill Gap ───────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct FillGapArgs {
    /// Entry ID or UUID prefix; omit to use the active entry
    pub id: Option<String>,
}

// ── Comment subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum CommentCommands {
    /// Add a comment to the active entry
    #[command(alias = "a")]
    Add(CommentAddArgs),
    /// List comments for an entry (defaults to active entry)
    #[command(alias = "ls")]
    List(CommentListArgs),
    /// Edit a comment
    #[command(alias = "e")]
    Edit(CommentEditArgs),
    /// Delete a comment
    #[command(aliases = ["d", "rm"])]
    Delete(CommentDeleteArgs),
}

#[derive(Args)]
pub struct CommentAddArgs {
    /// Entry ID or UUID prefix (defaults to active entry)
    #[arg(short, long)]
    pub entry: Option<String>,
    /// Comment text (multiple words without quotes)
    #[arg(num_args = 1..)]
    pub body: Vec<String>,
}

#[derive(Args)]
pub struct CommentListArgs {
    /// Entry ID or UUID prefix (defaults to active entry)
    pub entry: Option<String>,
}

#[derive(Args)]
pub struct CommentEditArgs {
    /// Comment ID or UUID prefix
    pub id: String,
    /// New comment text
    #[arg(num_args = 1..)]
    pub body: Vec<String>,
}

#[derive(Args)]
pub struct CommentDeleteArgs {
    /// Comment ID or UUID prefix
    pub id: String,
    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

// ── Event subcommands ─────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum EventCommands {
    /// Record a point-in-time event
    #[command(alias = "a")]
    Add(EventAddArgs),
    /// List events
    #[command(alias = "ls")]
    List(EventListArgs),
    /// Edit an existing event
    #[command(alias = "e")]
    Edit(EventEditArgs),
    /// Delete an event
    #[command(aliases = ["d", "rm"])]
    Delete(EventDeleteArgs),
}

#[derive(Args)]
pub struct EventAddArgs {
    /// Timestamp of the event — natural language or ISO 8601 (default: now)
    #[arg(long)]
    pub at: Option<String>,

    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
    pub task: Option<String>,

    #[arg(short, long)]
    pub note: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Args)]
pub struct EventListArgs {
    /// Start of date range (natural language or ISO 8601)
    #[arg(long)]
    pub from: Option<String>,

    /// End of date range (natural language or ISO 8601)
    #[arg(long)]
    pub until: Option<String>,

    /// Filter by project name
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Filter by tag (can be specified multiple times)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Maximum number of events to show
    #[arg(short, long)]
    pub limit: Option<u32>,
}

#[derive(Args)]
pub struct EventEditArgs {
    /// Event ID or UUID prefix
    pub id: String,

    /// New timestamp — natural language or ISO 8601
    #[arg(long)]
    pub at: Option<String>,

    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
    pub task: Option<String>,

    #[arg(short, long)]
    pub note: Option<String>,

    /// Replace all tags (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
}

#[derive(Args)]
pub struct EventDeleteArgs {
    /// Event ID or UUID prefix
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

// ── Import ────────────────────────────────────────────────────────────────────

#[derive(Args)]
#[command(long_about = "\
Import projects, tasks, and time entries from a CSV file.

Required columns (at least one):
  start           Combined start datetime
  start_date      Start date (pair with start_time or defaults to midnight)
  start_time      Start time (used with start_date)

Optional columns:
  project         Project name (created if new)
  task            Task name within the project (created if new)
  end / end_date / end_time
                  End datetime; may be split into end_date + end_time
  duration        Duration instead of end time (e.g. \"1:30:00\", \"1h30m\", \"90m\")
  note / description / comment
                  Free-text note
  tags            Comma-separated tags

Column names are matched case-insensitively; spaces and hyphens are treated as
underscores.  Naive datetimes (no timezone) are interpreted as local time.

File format is detected from the extension (.json → JSON, anything else → CSV).
Use -f json to force JSON when reading from stdin.

Examples:
  tmkpr import entries.csv
  tmkpr import entries.json
  tmkpr import -                          # CSV from stdin
  tmkpr -f json import -                  # JSON from stdin
  tmkpr export | tmkpr import -           # pipe export directly into import
")]
pub struct ImportArgs {
    /// Path to the CSV or JSON file, or '-' to read from stdin (default: stdin)
    pub file: Option<std::path::PathBuf>,

    /// Continue past row errors instead of aborting on the first failure
    #[arg(long)]
    pub skip_errors: bool,

    /// Show what would be imported without writing to the database
    #[arg(long)]
    pub dry_run: bool,
}

// ── Export ────────────────────────────────────────────────────────────────────

#[derive(Args)]
#[command(long_about = "\
Export time entries to a CSV file compatible with 'tmkpr import'.

Output columns: project, task, start, end, note, tags

Datetimes are written in local time (YYYY-MM-DD HH:MM:SS) so the file round-trips
cleanly through 'tmkpr import'.  Active entries are included by default with an
empty 'end' field.

When no file is given the CSV is written to stdout.

Examples:
  tmkpr export                         # all entries to stdout
  tmkpr export entries.csv             # all entries to file
  tmkpr export --from 2024-01-01 out.csv
  tmkpr export -p \"My Project\" out.csv
")]
pub struct ExportArgs {
    /// Output file (omit to write to stdout)
    pub file: Option<std::path::PathBuf>,

    /// Filter by project name
    #[arg(short, long, add = ArgValueCompleter::new(complete_projects))]
    pub project: Option<String>,

    /// Filter by task name
    #[arg(short, long, add = ArgValueCompleter::new(complete_tasks))]
    pub task: Option<String>,

    /// Start of date range (natural language or ISO 8601)
    #[arg(long)]
    pub from: Option<String>,

    /// End of date range (natural language or ISO 8601)
    #[arg(long)]
    pub until: Option<String>,

    /// Filter by tag (can be specified multiple times)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Exclude the currently active (unfinished) entry
    #[arg(long)]
    pub no_active: bool,
}

// ── Launch (ui / pomodoro) ────────────────────────────────────────────────────

#[derive(Args)]
pub struct LaunchArgs {
    /// Extra arguments passed directly to the launched binary
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
