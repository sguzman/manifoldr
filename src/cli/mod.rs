use clap::{Parser, Subcommand};

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
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    },
    /// Get user portfolio metrics
    Portfolio {
        /// Username or ID of the user
        user_id: Option<String>,
    },
    /// Get user portfolio history
    History {
        /// Username or ID of the user
        user_id: Option<String>,
        #[arg(short, long, default_value = "allTime")]
        period: String,
    },
    /// Get user's current positions
    Positions {
        /// Username or ID of the user
        user_id: Option<String>,
        #[arg(short, long, default_value_t = 10)]
        limit: i32,
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
    },
}
