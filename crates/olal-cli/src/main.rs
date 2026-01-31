//! Olal CLI - Your Personal Second Brain & Life Operating System

mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Olal - Your Personal Second Brain & Life Operating System
#[derive(Parser)]
#[command(name = "olal")]
#[command(author = "Lalo Morales <lalomorales22@github.com>")]
#[command(version)]
#[command(about = "Your Personal Second Brain & Life Operating System", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Olal (create config and database)
    Init,

    /// Manage configuration
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Show processing queue status
    Status,

    /// Show database statistics
    Stats,

    /// List recent items
    Recent {
        /// Maximum number of items to show
        #[arg(short, long, default_value = "10")]
        limit: i64,

        /// Filter by type (video, document, note, code, image)
        #[arg(short = 't', long)]
        item_type: Option<String>,
    },

    /// Search the knowledge base
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: i64,

        /// Use semantic (vector) search instead of full-text
        #[arg(long)]
        semantic: bool,
    },

    /// Ask a question using RAG (retrieval-augmented generation)
    Ask {
        /// Your question
        question: String,

        /// Model to use for generation (default: from config)
        #[arg(short, long)]
        model: Option<String>,

        /// Show source references
        #[arg(short, long, default_value = "true")]
        sources: bool,

        /// Maximum number of context chunks to use
        #[arg(short, long, default_value = "5")]
        context: usize,

        /// Stream the response as it's generated
        #[arg(long)]
        stream: bool,
    },

    /// Generate embeddings for semantic search
    Embed {
        /// Embed all unembedded chunks
        #[arg(long)]
        all: bool,

        /// Embed chunks for a specific item (ID or prefix)
        #[arg(short, long)]
        item: Option<String>,

        /// Batch size for processing
        #[arg(long, default_value = "10")]
        batch_size: usize,
    },

    /// Show details of an item
    Show {
        /// Item ID
        id: String,
    },

    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCommands),

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Add a tag to an item
    Tag {
        /// Item ID
        item_id: String,

        /// Tag name
        tag: String,
    },

    /// List all tags
    Tags,

    /// Ingest files or directories
    Ingest {
        /// Path to file or directory to ingest
        path: String,

        /// Filter by file type (video, document, note, code, image)
        #[arg(short = 't', long)]
        item_type: Option<String>,

        /// Show what would be ingested without actually ingesting
        #[arg(long)]
        dry_run: bool,

        /// Add to processing queue instead of processing immediately
        #[arg(short, long)]
        queue: bool,
    },

    /// Capture a quick thought or note
    Capture {
        /// The thought or note content
        thought: String,

        /// Optional title for the note
        #[arg(short, long)]
        title: Option<String>,

        /// Tags to add (can be specified multiple times)
        #[arg(short = 'T', long = "tag")]
        tags: Vec<String>,
    },

    /// Detect engaging clips from video/audio content
    Clips {
        /// Item ID (video or audio with transcript)
        item_id: String,

        /// Number of clips to suggest
        #[arg(short, long, default_value = "5")]
        count: usize,

        /// Minimum clip duration in seconds
        #[arg(long, default_value = "30")]
        min_duration: u32,

        /// Maximum clip duration in seconds
        #[arg(long, default_value = "90")]
        max_duration: u32,

        /// Model to use for analysis
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Watch directories for new files
    #[command(subcommand)]
    Watch(WatchCommands),

    /// Generate YouTube metadata from video content
    Youtube {
        /// Item ID (video with transcript)
        item_id: String,

        /// Content style: tutorial, review, vlog, educational
        #[arg(short, long)]
        style: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,

        /// Generate title only
        #[arg(long)]
        title_only: bool,

        /// Generate description only
        #[arg(long)]
        description_only: bool,

        /// Generate chapters only
        #[arg(long)]
        chapters_only: bool,

        /// Generate tags only
        #[arg(long)]
        tags_only: bool,
    },

    /// Start an interactive shell
    Shell,

    /// Generate a digest of recent content
    Digest {
        /// Time period: day, week, month
        #[arg(short, long, default_value = "day")]
        period: String,

        /// Start from specific date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Output to file
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Open config file in editor
    Edit,

    /// Add a directory to watch
    AddWatch {
        /// Directory path
        path: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., ollama.model)
        key: String,

        /// Value to set
        value: String,
    },
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Add a new task
    Add {
        /// Task description
        description: String,

        /// Priority (higher = more important)
        #[arg(short, long, default_value = "0")]
        priority: i32,

        /// Project name
        #[arg(short = 'P', long)]
        project: Option<String>,
    },

    /// List tasks
    List {
        /// Filter by status (pending, in_progress, done)
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Mark a task as done
    Done {
        /// Task ID
        id: String,
    },

    /// Delete a task
    Delete {
        /// Task ID
        id: String,
    },
}

#[derive(Subcommand)]
enum WatchCommands {
    /// Start watching directories (foreground)
    Start {
        /// Run as daemon (background)
        #[arg(short, long)]
        daemon: bool,
    },

    /// Stop the watch daemon
    Stop,

    /// Show watch configuration and status
    Status,
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// Create a new project
    Create {
        /// Project name
        name: String,

        /// Project description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List all projects
    List,

    /// Show project details
    Show {
        /// Project name or ID
        name: String,
    },
}

fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("olal=debug,info"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("olal=info,warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}

fn main() {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Config(cmd) => match cmd {
            ConfigCommands::Show => commands::config::show(),
            ConfigCommands::Edit => commands::config::edit(),
            ConfigCommands::AddWatch { path } => commands::config::add_watch(&path),
            ConfigCommands::Set { key, value } => commands::config::set(&key, &value),
        },
        Commands::Status => commands::status::run(),
        Commands::Stats => commands::stats::run(),
        Commands::Recent { limit, item_type } => commands::recent::run(limit, item_type),
        Commands::Search { query, limit, semantic } => commands::search::run(&query, limit, semantic),
        Commands::Show { id } => commands::show::run(&id),
        Commands::Ask {
            question,
            model,
            sources,
            context,
            stream,
        } => commands::ask::run(&question, model, sources, context, stream),
        Commands::Embed {
            all,
            item,
            batch_size,
        } => commands::embed::run(all, item, batch_size),
        Commands::Task(cmd) => match cmd {
            TaskCommands::Add {
                description,
                priority,
                project,
            } => commands::task::add(&description, priority, project),
            TaskCommands::List { status } => commands::task::list(status),
            TaskCommands::Done { id } => commands::task::done(&id),
            TaskCommands::Delete { id } => commands::task::delete(&id),
        },
        Commands::Project(cmd) => match cmd {
            ProjectCommands::Create { name, description } => {
                commands::project::create(&name, description)
            }
            ProjectCommands::List => commands::project::list(),
            ProjectCommands::Show { name } => commands::project::show(&name),
        },
        Commands::Tag { item_id, tag } => commands::tag::add(&item_id, &tag),
        Commands::Tags => commands::tag::list(),
        Commands::Ingest {
            path,
            item_type,
            dry_run,
            queue,
        } => commands::ingest::run(&path, item_type, dry_run, queue),
        Commands::Capture {
            thought,
            title,
            tags,
        } => commands::capture::run(&thought, title, tags),
        Commands::Clips {
            item_id,
            count,
            min_duration,
            max_duration,
            model,
        } => commands::clips::run(&item_id, count, min_duration, max_duration, model),
        Commands::Shell => commands::shell::run(),
        Commands::Watch(cmd) => match cmd {
            WatchCommands::Start { daemon } => commands::watch::run(daemon),
            WatchCommands::Stop => commands::watch::stop(),
            WatchCommands::Status => commands::watch::status(),
        },
        Commands::Youtube {
            item_id,
            style,
            model,
            title_only,
            description_only,
            chapters_only,
            tags_only,
        } => commands::youtube::run(
            &item_id,
            style,
            model,
            title_only,
            description_only,
            chapters_only,
            tags_only,
        ),
        Commands::Digest {
            period,
            since,
            output,
            model,
        } => commands::digest::run(&period, since, output, model),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}
