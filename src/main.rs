mod api;
mod cli;
mod logging;
mod utils;
#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, UserCommands, MarketCommands, BetCommands};
use api::ManifoldClient;
use dotenvy::dotenv;
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _log_guard = logging::init_logging();

    let cli = Cli::parse();
    
    let api_key = cli.api_key.or_else(|| std::env::var("API_KEY").ok());
    let client = ManifoldClient::new(api_key);

    match cli.command {
        Commands::User { command } => handle_user_command(client, command).await?,
        Commands::Market { command } => handle_market_command(client, command).await?,
        Commands::Bet { command } => handle_bet_command(client, command).await?,
    }

    Ok(())
}

#[instrument(skip(client))]
async fn handle_user_command(client: ManifoldClient, command: UserCommands) -> Result<()> {
    match command {
        UserCommands::Me => {
            info!("Fetching current user info");
            let user = client.get_me().await?;
            utils::print_user_table(&user);
        }
        UserCommands::Get { username_or_id } => {
            info!(username_or_id, "Fetching user info");
            let user = if username_or_id.len() == 22 { // Typical ID length
                client.get_user_by_id(&username_or_id).await?
            } else {
                client.get_user_by_username(&username_or_id).await?
            };
            utils::print_user_table(&user);
        }
        UserCommands::Portfolio { user_id } => {
            info!(user_id, "Fetching portfolio metrics");
            let metrics = client.get_user_portfolio(&user_id).await?;
            println!("{}", serde_json::to_string_pretty(&metrics)?);
        }
        UserCommands::History { user_id, period } => {
            info!(user_id, period, "Fetching portfolio history");
            let history = client.get_user_portfolio_history(&user_id, &period).await?;
            utils::print_portfolio_history_table(&history);
        }
        UserCommands::Positions { user_id, limit } => {
            info!(user_id, limit, "Fetching user positions");
            let response = client.get_user_contract_metrics(&user_id, limit).await?;
            let mut all_metrics = Vec::new();
            for metrics in response.metrics_by_contract.values() {
                all_metrics.extend(metrics.clone());
            }
            utils::print_positions_table(&all_metrics);
        }
    }
    Ok(())
}

#[instrument(skip(client))]
async fn handle_market_command(client: ManifoldClient, command: MarketCommands) -> Result<()> {
    match command {
        MarketCommands::List { limit, sort, order, before } => {
            info!(limit, ?sort, ?order, ?before, "Listing markets");
            let markets = client.list_markets(Some(limit), sort.as_deref(), order.as_deref(), before.as_deref()).await?;
            utils::print_markets_table(&markets);
        }
        MarketCommands::Search { term, limit, sort, filter } => {
            info!(term, limit, ?sort, ?filter, "Searching markets");
            let markets = client.search_markets(&term, Some(limit), sort.as_deref(), filter.as_deref()).await?;
            utils::print_markets_table(&markets);
        }
        MarketCommands::Get { id_or_slug, slug } => {
            info!(id_or_slug, slug, "Fetching market info");
            let market = if slug {
                client.get_market_by_slug(&id_or_slug).await?
            } else {
                client.get_market_by_id(&id_or_slug).await?
            };
            println!("{}", serde_json::to_string_pretty(&market)?);
        }
        MarketCommands::Positions { market_id, top, bottom } => {
            info!(market_id, ?top, ?bottom, "Fetching market positions");
            let positions = client.get_market_positions(&market_id, top, bottom).await?;
            utils::print_positions_table(&positions);
        }
    }
    Ok(())
}

#[instrument(skip(client))]
async fn handle_bet_command(client: ManifoldClient, command: BetCommands) -> Result<()> {
    match command {
        BetCommands::Place { market_id, amount, outcome } => {
            info!(market_id, amount, outcome, "Placing bet");
            let result = client.place_bet(&market_id, amount, &outcome).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        BetCommands::List { user_id, market_id, limit } => {
            info!(?user_id, ?market_id, limit, "Listing bets");
            let bets = client.list_bets(user_id.as_deref(), market_id.as_deref(), Some(limit)).await?;
            utils::print_bets_table(&bets);
        }
    }
    Ok(())
}
