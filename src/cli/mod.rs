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
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// User related commands
    User {
        #[command(subcommand)]
        command: UserCommands,
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
        user_id: Option<String> 
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
        max_width: Option<usize>,
        /// Number of positions to display in the table
        #[arg(short = 'd', long)]
        display_limit: Option<usize>,
    },
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
    },
    /// Get a specific market
    Get {
        /// Market ID or Slug
        id_or_slug: String,
        #[arg(short, long)]
        slug: bool,
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
        max_width: Option<usize>,
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
        /// Outcome (e.g., YES, NO)
        outcome: String,
    },
    /// List bets
    List {
        #[arg(short, long)]
        user_id: Option<String>,
        #[arg(short, long)]
        market_id: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: i32,
    },
}
