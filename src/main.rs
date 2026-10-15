mod api;
mod cli;
mod logging;
#[cfg(test)]
#[path = "tests.rs"]
mod fixture_tests;
mod utils;

use crate::api::client::{ManifoldClient, is_not_found};
use crate::api::models::ContractMetric;
use anyhow::{Result, bail};
use clap::Parser;
use cli::{BetCommands, Cli, Commands, MarketCommands, UserCommands};
use dotenvy::dotenv;
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _log_guard = logging::init_logging();
    let cli = Cli::parse();
    let json = cli.json;
    let api_key = cli.api_key.or_else(|| std::env::var("API_KEY").ok());
    let client = ManifoldClient::new(api_key);

    match cli.command {
        Commands::User { command } => handle_user_command(client, command, json).await?,
        Commands::Market { command } => handle_market_command(client, command, json).await?,
        Commands::Bet { command } => handle_bet_command(client, command, json).await?,
    }
    Ok(())
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

async fn resolve_user_id(client: &ManifoldClient, identifier: Option<String>) -> Result<String> {
    if let Some(identifier) = identifier {
        match client.get_user_by_username(&identifier).await {
            Ok(user) => Ok(user.id),
            Err(err) if is_not_found(&err) => Ok(client.get_user_by_id(&identifier).await?.id),
            Err(err) => Err(err),
        }
    } else {
        Ok(client.get_me().await?.id)
    }
}

#[instrument(skip(client))]
async fn handle_user_command(
    client: ManifoldClient,
    command: UserCommands,
    json: bool,
) -> Result<()> {
    match command {
        UserCommands::Me => {
            let user = client.get_me().await?;
            if json {
                print_json(&user)?;
            } else {
                utils::print_user_table(&user);
            }
        }
        UserCommands::Get { username_or_id } => {
            let user = match client.get_user_by_username(&username_or_id).await {
                Ok(user) => user,
                Err(err) if is_not_found(&err) => client.get_user_by_id(&username_or_id).await?,
                Err(err) => return Err(err),
            };
            if json {
                print_json(&user)?;
            } else {
                utils::print_user_table(&user);
            }
        }
        UserCommands::Portfolio { user_id } => {
            let id = resolve_user_id(&client, user_id).await?;
            print_json(&client.get_user_portfolio(&id).await?)?;
        }
        UserCommands::History { user_id, period } => {
            let id = resolve_user_id(&client, user_id).await?;
            let history = client.get_user_portfolio_history(&id, &period).await?;
            if json {
                print_json(&history)?;
            } else {
                utils::print_portfolio_history_table(&history);
            }
        }
        UserCommands::Positions {
            user_id,
            limit,
            watch,
            max_width,
            display_limit,
            all,
        } => {
            if limit <= 0 {
                bail!("Position limit must be positive");
            }
            if watch.is_some_and(|interval| interval < 2) {
                bail!("--watch must be at least 2 seconds to respect API rate limits");
            }
            let id = resolve_user_id(&client, user_id).await?;
            loop {
                // Never print terminal control sequences in JSON mode.
                if watch.is_some() && !json {
                    print!("\x1B[2J\x1B[1;1H");
                    eprintln!(
                        "Positions for {id} | {}",
                        chrono::Local::now().format("%H:%M:%S")
                    );
                }
                let fetch_limit =
                    limit.max(display_limit.unwrap_or(0).min(i32::MAX as usize) as i32);
                info!(id, fetch_limit, "Fetching positions");
                let response = client.get_user_contract_metrics(&id, fetch_limit).await?;
                if json {
                    print_json(&response)?;
                } else {
                    let mut groups: Vec<Vec<ContractMetric>> =
                        response.metrics_by_contract.values().cloned().collect();
                    groups.sort_by(|a, b| {
                        let profit = |group: &Vec<ContractMetric>| -> f64 {
                            group.iter().map(|m| m.profit.unwrap_or_default()).sum()
                        };
                        profit(b).total_cmp(&profit(a))
                    });
                    let titles = response
                        .contracts
                        .into_iter()
                        .map(|m| (m.id, m.question))
                        .collect();
                    utils::print_positions_table(
                        &groups,
                        Some(&titles),
                        max_width,
                        display_limit,
                        all,
                    );
                }
                if let Some(seconds) = watch {
                    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[instrument(skip(client))]
async fn handle_market_command(
    client: ManifoldClient,
    command: MarketCommands,
    json: bool,
) -> Result<()> {
    match command {
        MarketCommands::List {
            limit,
            sort,
            order,
            before,
        } => {
            if !(1..=1000).contains(&limit) {
                bail!("Market limit must be between 1 and 1000");
            }
            let markets = client
                .list_markets(
                    Some(limit),
                    sort.as_deref(),
                    order.as_deref(),
                    before.as_deref(),
                )
                .await?;
            if json {
                print_json(&markets)?;
            } else {
                utils::print_markets_table(&markets);
            }
        }
        MarketCommands::Search {
            term,
            limit,
            sort,
            filter,
        } => {
            if limit <= 0 {
                bail!("Search limit must be positive");
            }
            let markets = client
                .search_markets(&term, Some(limit), sort.as_deref(), filter.as_deref())
                .await?;
            if json {
                print_json(&markets)?;
            } else {
                utils::print_markets_table(&markets);
            }
        }
        MarketCommands::Get { id_or_slug, slug } => {
            let market = if slug {
                client.get_market_by_slug(&id_or_slug).await?
            } else {
                client.get_market_by_id(&id_or_slug).await?
            };
            print_json(&market)?;
        }
        MarketCommands::Probabilities { ids } => {
            print_json(&client.market_probabilities(&ids).await?)?;
        }
        MarketCommands::Positions {
            market_id,
            top,
            bottom,
            max_width,
        } => {
            let positions = client.get_market_positions(&market_id, top, bottom).await?;
            if json {
                print_json(&positions)?;
            } else {
                let mut groups_map: std::collections::HashMap<String, Vec<ContractMetric>> =
                    std::collections::HashMap::new();
                for position in positions {
                    groups_map
                        .entry(position.user_id.clone())
                        .or_default()
                        .push(position);
                }
                let mut groups: Vec<Vec<ContractMetric>> = groups_map.into_values().collect();
                groups.sort_by(|a, b| {
                    let profit = |group: &Vec<ContractMetric>| -> f64 {
                        group.iter().map(|p| p.profit.unwrap_or_default()).sum()
                    };
                    profit(b).total_cmp(&profit(a))
                });
                utils::print_positions_table(&groups, None, max_width, None, true);
            }
        }
    }
    Ok(())
}

#[instrument(skip(client))]
async fn handle_bet_command(
    client: ManifoldClient,
    command: BetCommands,
    json: bool,
) -> Result<()> {
    match command {
        BetCommands::Place {
            market_id,
            amount,
            outcome,
            execute,
        } => {
            if execute {
                eprintln!("Executing a REAL trade on Manifold.");
            } else {
                eprintln!("SIMULATION ONLY: Manifold dryRun=true; no bet will be placed.");
            }
            print_json(
                &client
                    .place_bet(&market_id, amount, &outcome, execute)
                    .await?,
            )?;
        }
        BetCommands::List {
            user_id,
            market_id,
            limit,
        } => {
            if limit <= 0 {
                bail!("Bet limit must be positive");
            }
            let bets = client
                .list_bets(user_id.as_deref(), market_id.as_deref(), Some(limit))
                .await?;
            if json {
                print_json(&bets)?;
            } else {
                utils::print_bets_table(&bets);
            }
        }
    }
    Ok(())
}
