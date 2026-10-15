use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(name = "manifoldr")]
#[command(about = "CLI tool for Manifold Markets", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Set the API key (can also be set via MANIFOLD_API_KEY env var)
    #[arg(short, long, env = "MANIFOLD_API_KEY")]
    pub api_key: Option<String>,

    /// Print machine-readable JSON instead of formatted tables
    #[arg(long, global = true)]
    pub json: bool,

    /// Use only locally cached public API data; never issue network requests
    #[arg(long, global = true, conflicts_with = "no_cache")]
    pub offline: bool,

    /// Disable public-read caching entirely
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Cache TTL in seconds for public reads (default 60 seconds)
    #[arg(long, global = true, default_value_t = 60)]
    pub cache_ttl: u64,

    /// Override public cache location (defaults to XDG_CACHE_HOME/manifoldr)
    #[arg(long, global = true)]
    pub cache_dir: Option<std::path::PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Emit shell completion definitions for bash, fish, zsh, etc.
    Completions { shell: Shell },
    /// Subscribe to live Manifold WebSocket events as NDJSON
    Stream {
        /// One or more documented stream topics (repeat for multiple)
        #[arg(long = "topic", required = true, num_args = 1..)]
        topics: Vec<String>,
        /// Stop after receiving this many events (0 = continuous)
        #[arg(long, default_value_t = 0)]
        count: usize,
    },
    /// User related commands
    User {
        #[command(subcommand)]
        command: UserCommands,
    },
    /// Public topic discovery
    Topic {
        #[command(subcommand)]
        command: TopicCommands,
    },
    /// Market related commands
    Market {
        #[command(subcommand)]
        command: MarketCommands,
    },
    /// Bet related commands
    Bet {
        #[command(subcommand)]
        command: BetCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// List public users (cursor is the previous page's final user ID)
    List {
        #[arg(long, default_value_t = 20)]
        limit: i32,
        #[arg(long)]
        before: Option<String>,
    },
    /// Get information about the authenticated user
    Me,
    /// Get information about a specific user
    Get {
        /// Username or ID of the user
        username_or_id: String,
        /// Treat the argument directly as an immutable user ID
        #[arg(long)]
        id: bool,
    },
    /// Get user portfolio metrics
    Portfolio {
        /// Username or ID of the user
        user_id: Option<String>,
        /// Treat the argument as an immutable user ID
        #[arg(long, requires = "user_id")]
        id: bool,
    },
    /// Get user portfolio history
    History {
        /// Username or ID of the user
        user_id: Option<String>,
        #[arg(short, long, default_value = "allTime")]
        period: String,
        #[arg(long, requires = "user_id")]
        id: bool,
    },
    /// Get user's current positions
    Positions {
        /// Username or ID of the user
        user_id: Option<String>,
        #[arg(short, long, default_value_t = 10)]
        limit: i32,
        /// Treat the argument as an immutable user ID
        #[arg(long, requires = "user_id")]
        id: bool,
        /// Number of contracts to skip in server-side pagination
        #[arg(long, default_value_t = 0)]
        offset: u32,
        /// Order of server-side pages
        #[arg(long, default_value = "lastBetTime", value_parser = ["lastBetTime", "profit"])]
        order: String,
        /// Include per-answer positions in multiple-choice markets
        #[arg(long)]
        per_answer: bool,
        /// Watch and refresh positions every N seconds
        #[arg(short, long, default_missing_value = "10", num_args = 0..=1)]
        watch: Option<u64>,
        /// Max width of market title
        #[arg(short, long)]
        max_width: Option<u16>,
        /// Number of positions to display in the table
        #[arg(short = 'd', long)]
        display_limit: Option<usize>,
        /// Include zero-value (closed) positions
        #[arg(short, long)]
        all: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TopicCommands {
    /// List public topics using optional millisecond timestamp cursor
    List {
        #[arg(long)]
        before_time: Option<i64>,
    },
    /// Get topic by slug
    Get { slug: String },
}

#[derive(Subcommand, Debug)]
pub enum MarketCommands {
    /// Preview selling shares (or actually sell with --execute)
    Sell {
        market_id: String,
        /// Sell this many shares; mutually exclusive with --all
        #[arg(long, required_unless_present = "all", conflicts_with = "all")]
        shares: Option<f64>,
        /// Explicitly sell all shares in the outcome
        #[arg(long, conflicts_with = "shares")]
        all: bool,
        /// Which shares to sell
        #[arg(long, value_parser = ["YES", "NO"])]
        outcome: String,
        /// Required by Manifold for multiple-choice markets
        #[arg(long)]
        answer_id: Option<String>,
        /// Send a real sale request; otherwise preview locally
        #[arg(long)]
        execute: bool,
    },
    /// List markets
    List {
        #[arg(short, long, default_value_t = 10)]
        limit: i32,
        #[arg(short, long)]
        sort: Option<String>,
        #[arg(short, long)]
        order: Option<String>,
        #[arg(short, long)]
        before: Option<String>,
        /// Filter by creator's immutable user ID
        #[arg(long)]
        user_id: Option<String>,
        /// Filter by topic ID
        #[arg(long)]
        group_id: Option<String>,
    },
    /// Search for markets
    Search {
        /// Search term
        term: String,
        #[arg(short, long, default_value_t = 10)]
        limit: i32,
        #[arg(short, long)]
        sort: Option<String>,
        #[arg(short, long)]
        filter: Option<String>,
        /// Unix milliseconds of last result; requires --sort newest
        #[arg(long)]
        before_time: Option<i64>,
        /// Offset (maximum 1000), incompatible with beforeTime
        #[arg(long, default_value_t = 0, conflicts_with = "before_time")]
        offset: u32,
        /// Filter to a contract type such as BINARY
        #[arg(long)]
        contract_type: Option<String>,
        /// Filter by topic slug
        #[arg(long)]
        topic_slug: Option<String>,
        /// Filter by immutable creator ID
        #[arg(long)]
        creator_id: Option<String>,
    },
    /// Get a specific market
    Get {
        /// Market ID or Slug
        id_or_slug: String,
        #[arg(short, long)]
        slug: bool,
    },
    /// Retrieve a market's current probability (or per-answer probabilities)
    Probability { id: String },
    /// Get probability data for up to 100 market IDs in one API request
    Probabilities {
        #[arg(required = true, num_args = 1..=100)]
        ids: Vec<String>,
    },
    /// Get market positions
    Positions {
        market_id: String,
        #[arg(short, long)]
        top: Option<i32>,
        #[arg(short, long)]
        bottom: Option<i32>,
        /// Max width of market title
        #[arg(short, long)]
        max_width: Option<u16>,
    },
}

#[derive(Subcommand, Debug)]
pub enum BetCommands {
    /// Preview a limit-order cancellation, or execute it with --execute
    Cancel {
        bet_id: String,
        #[arg(long)]
        execute: bool,
    },
    /// Place a bet
    Place {
        /// Market ID
        market_id: String,
        /// Amount to bet
        amount: f64,
        /// Outcome (YES or NO for binary markets)
        outcome: String,
        /// Execute a real trade; without this, Manifold simulates the trade
        #[arg(long)]
        execute: bool,
    },
    /// List bets
    List {
        #[arg(short, long)]
        user_id: Option<String>,
        #[arg(short, long)]
        market_id: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: i32,
        /// Fetch older bets before the given bet ID
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        /// Fetch newer bets after the given bet ID
        #[arg(long)]
        after: Option<String>,
        #[arg(long, conflicts_with = "user_id")]
        username: Option<String>,
        #[arg(long, conflicts_with = "market_id")]
        contract_slug: Option<String>,
        /// Return open limit orders, including closed/resolved-market orders
        #[arg(long)]
        open_limit: bool,
        #[arg(long)]
        before_time: Option<i64>,
        #[arg(long)]
        after_time: Option<i64>,
        #[arg(long, value_parser = ["asc", "desc"])]
        order: Option<String>,
    },
}
