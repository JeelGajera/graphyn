use clap::{Parser, Subcommand};

mod commands;
mod output;

#[derive(Parser)]
#[command(
    name = "graphyn",
    version,
    about = "⚡ Understand the blast radius before you pull the trigger.",
    long_about = "\
Graphyn is a code intelligence engine that models your codebase as a \
living graph of symbol relationships, so coding agents and developers \
know exactly what will break before making a change.\n\
\n\
QUICK START:\n  \
  graphyn analyze ./my-repo\n  \
  graphyn query blast-radius UserPayload\n  \
  graphyn query usages UserPayload\n  \
  graphyn status",
    author = "Graphyn Contributors"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a codebase and build the symbol relationship graph
    Analyze {
        /// Path to the repository root
        #[arg(default_value = ".")]
        path: String,
        /// Comma-separated include patterns (relative paths/globs)
        #[arg(long)]
        include: Option<String>,
        /// Comma-separated exclude patterns (relative paths/globs)
        #[arg(long)]
        exclude: Option<String>,
        /// Disable .gitignore filtering
        #[arg(long)]
        no_gitignore: bool,
    },

    /// Query the symbol relationship graph
    Query {
        #[command(subcommand)]
        subcommand: QueryCommands,
    },

    /// Watch for file changes and update the graph incrementally
    Watch {
        /// Path to the repository root
        #[arg(default_value = ".")]
        path: String,
        /// Comma-separated include patterns (relative paths/globs)
        #[arg(long)]
        include: Option<String>,
        /// Comma-separated exclude patterns (relative paths/globs)
        #[arg(long)]
        exclude: Option<String>,
        /// Disable .gitignore filtering
        #[arg(long)]
        no_gitignore: bool,
    },

    /// Start the MCP server for agent integration
    Serve {
        /// Port number for TCP transport
        #[arg(long, default_value = "7700")]
        port: u16,

        /// Use stdio transport instead of TCP
        #[arg(long)]
        stdio: bool,
    },

    /// Show graph statistics and status
    Status {
        /// Path to the repository root
        #[arg(default_value = ".")]
        path: String,
    },
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Find all symbols that depend on the target — what will break?
    #[command(name = "blast-radius")]
    BlastRadius {
        /// Symbol name to analyze
        symbol: String,

        /// Narrow to a specific file if symbol name is ambiguous
        #[arg(long, short)]
        file: Option<String>,

        /// Traversal depth (default: 3, max: 10)
        #[arg(long, short, default_value = "3")]
        depth: usize,

        /// Path to the repository root
        #[arg(long, default_value = ".")]
        path: String,
    },

    /// Find every usage of a symbol, including under aliases
    #[command(name = "usages")]
    Usages {
        /// Symbol name to find
        symbol: String,

        /// Narrow to a specific file if symbol name is ambiguous
        #[arg(long, short)]
        file: Option<String>,

        /// Path to the repository root
        #[arg(long, default_value = ".")]
        path: String,
    },

    /// Find all dependencies of a symbol
    #[command(name = "deps")]
    Deps {
        /// Symbol name to analyze
        symbol: String,

        /// Narrow to a specific file if symbol name is ambiguous
        #[arg(long, short)]
        file: Option<String>,

        /// Traversal depth (default: 3, max: 10)
        #[arg(long, short, default_value = "3")]
        depth: usize,

        /// Path to the repository root
        #[arg(long, default_value = ".")]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Analyze {
            path,
            include,
            exclude,
            no_gitignore,
        } => commands::analyze::run(&path, include.as_deref(), exclude.as_deref(), !no_gitignore),

        Commands::Query { subcommand } => match subcommand {
            QueryCommands::BlastRadius {
                symbol,
                file,
                depth,
                path,
            } => commands::query::run_blast_radius(&symbol, file.as_deref(), depth, &path),
            QueryCommands::Usages { symbol, file, path } => {
                commands::query::run_usages(&symbol, file.as_deref(), &path)
            }
            QueryCommands::Deps {
                symbol,
                file,
                depth,
                path,
            } => commands::query::run_deps(&symbol, file.as_deref(), depth, &path),
        },

        Commands::Watch {
            path,
            include,
            exclude,
            no_gitignore,
        } => commands::watch::run(&path, include.as_deref(), exclude.as_deref(), !no_gitignore),

        Commands::Serve { port, stdio } => commands::serve::run(port, stdio),

        Commands::Status { path } => commands::status::run(&path),
    };

    if let Err(e) = result {
        output::error(&e.to_string());
        std::process::exit(1);
    }
}
