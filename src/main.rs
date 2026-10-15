mod cli;
mod logging;
mod utils;

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser};
use cli::{BetCommands, Cli, Commands, MarketCommands, TopicCommands, UserCommands};
use dotenvy::dotenv;
use manifoldr::api::client::{BetFilters, ManifoldClient, is_not_found};
use manifoldr::api::models::ContractMetric;
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _log_guard = logging::init_logging();
    let cli = Cli::parse();
    let json = cli.json;
    if cli.cache_ttl > 86_400 {
        bail!("--cache-ttl must not exceed 86400 seconds");
    }
    let api_key = cli.api_key.or_else(|| std::env::var("API_KEY").ok());
    if let Commands::Completions { shell } = cli.command {
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            "manifoldr",
            &mut std::io::stdout(),
        );
        return Ok(());
    }
    if let Commands::Stream { topics, count } = cli.command {
        if cli.offline {
            bail!("--offline cannot subscribe to a live WebSocket stream");
        }
        manifoldr::realtime::run(topics, count).await?;
        return Ok(());
    }
    let mut client = ManifoldClient::new(api_key)?;
    if !cli.no_cache {
        let cache_root = cli.cache_dir.or_else(|| {
            std::env::var_os("XDG_CACHE_HOME")
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|home| std::path::PathBuf::from(home).join(".cache"))
                })
                .map(|root| root.join("manifoldr"))
        });
        if let Some(dir) = cache_root {
            client.configure_public_cache(
                dir,
                std::time::Duration::from_secs(cli.cache_ttl),
                cli.offline,
            );
        } else if cli.offline {
            bail!("--offline needs --cache-dir or HOME/XDG_CACHE_HOME");
        }
    }

    match cli.command {
        Commands::Stream { .. } | Commands::Completions { .. } => {
            unreachable!("handled before client initialization")
        }
        Commands::User { command } => handle_user_command(client, command, json).await?,
        Commands::Topic { command } => handle_topic_command(client, command).await?,
        Commands::Market { command } => handle_market_command(client, command, json).await?,
        Commands::Bet { command } => handle_bet_command(client, command, json).await?,
    }
    Ok(())
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

async fn resolve_user_id(
    client: &ManifoldClient,
    identifier: Option<String>,
    direct_id: bool,
) -> Result<String> {
    if direct_id {
        return identifier.ok_or_else(|| anyhow::anyhow!("Explicit user ID required"));
    }
    if let Some(identifier) = identifier {
        match client.get_user_by_username(&identifier).await {
            Ok(user) => Ok(user.id),
            Err(err) if is_not_found(&err) => Ok(client.get_user_by_id(&identifier).await?.id),
            Err(err) => Err(err.into()),
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
        UserCommands::List { limit, before } => {
            let users = client.list_users(limit, before.as_deref()).await?;
            if json {
                print_json(&users)?;
            } else {
                for user in &users {
                    println!("{}\t{}\t{}", user.id, user.username, user.name);
                }
            }
        }
        UserCommands::Me => {
            let user = client.get_me().await?;
            if json {
                print_json(&user)?;
            } else {
                utils::print_user_table(&user);
            }
        }
        UserCommands::Get { username_or_id, id } => {
            let user = if id {
                client.get_user_by_id(&username_or_id).await?
            } else {
                match client.get_user_by_username(&username_or_id).await {
                    Ok(user) => user,
                    Err(err) if is_not_found(&err) => {
                        client.get_user_by_id(&username_or_id).await?
                    }
                    Err(err) => return Err(err.into()),
                }
            };
            if json {
                print_json(&user)?;
            } else {
                utils::print_user_table(&user);
            }
        }
        UserCommands::Portfolio {
            user_id,
            id: direct_id,
        } => {
            let id = resolve_user_id(&client, user_id, direct_id).await?;
            print_json(&client.get_user_portfolio(&id).await?)?;
        }
        UserCommands::History {
            user_id,
            period,
            id: direct_id,
        } => {
            let id = resolve_user_id(&client, user_id, direct_id).await?;
            let history = client.get_user_portfolio_history(&id, &period).await?;
            if json {
                print_json(&history)?;
            } else {
                utils::print_portfolio_history_table(&history);
            }
        }
        UserCommands::Positions {
            user_id,
            id: direct_id,
            limit,
            offset,
            order,
            per_answer,
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
            let id = resolve_user_id(&client, user_id, direct_id).await?;
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
                let response = client
                    .get_user_contract_metrics_page(
                        &id,
                        fetch_limit,
                        offset,
                        Some(&order),
                        per_answer,
                    )
                    .await?;
                if json {
                    print_json(&response)?;
                } else {
                    let mut groups: Vec<Vec<ContractMetric>> =
                        response.metrics_by_contract.values().cloned().collect();
                    if order == "profit" {
                        groups.sort_by(|a, b| {
                            let profit = |group: &Vec<ContractMetric>| -> f64 {
                                group.iter().map(|m| m.profit.unwrap_or_default()).sum()
                            };
                            profit(b).total_cmp(&profit(a))
                        });
                    } else {
                        groups.sort_by(|a, b| {
                            let last = |group: &Vec<ContractMetric>| -> i64 {
                                group
                                    .iter()
                                    .filter_map(|m| m.last_bet_time)
                                    .max()
                                    .unwrap_or(0)
                            };
                            last(b).cmp(&last(a))
                        });
                    }
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
async fn handle_topic_command(client: ManifoldClient, command: TopicCommands) -> Result<()> {
    match command {
        TopicCommands::List { before_time } => print_json(&client.list_groups(before_time).await?)?,
        TopicCommands::Get { slug } => print_json(&client.get_group_by_slug(&slug).await?)?,
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
        MarketCommands::Sell {
            market_id,
            shares,
            all: _,
            outcome,
            answer_id,
            execute,
        } => {
            if execute {
                eprintln!("Executing a REAL share sale in market {market_id}.");
            } else {
                eprintln!("PREVIEW ONLY: no sale request will be sent.");
            }
            print_json(
                &client
                    .sell_shares(&market_id, &outcome, shares, answer_id.as_deref(), execute)
                    .await?,
            )?;
        }
        MarketCommands::List {
            limit,
            sort,
            order,
            before,
            user_id,
            group_id,
        } => {
            if !(1..=1000).contains(&limit) {
                bail!("Market limit must be between 1 and 1000");
            }
            let markets = client
                .list_markets_filtered(
                    Some(limit),
                    sort.as_deref(),
                    order.as_deref(),
                    before.as_deref(),
                    user_id.as_deref(),
                    group_id.as_deref(),
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
            before_time,
            offset,
            contract_type,
            topic_slug,
            creator_id,
        } => {
            if limit <= 0 {
                bail!("Search limit must be positive");
            }
            let markets = client
                .search_markets_advanced(
                    &term,
                    manifoldr::api::client::SearchFilters {
                        limit: Some(limit),
                        sort: sort.as_deref(),
                        filter: filter.as_deref(),
                        before_time,
                        offset,
                        contract_type: contract_type.as_deref(),
                        topic_slug: topic_slug.as_deref(),
                        creator_id: creator_id.as_deref(),
                    },
                )
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
        MarketCommands::Probability { id } => {
            print_json(&client.market_probability(&id).await?)?;
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
        BetCommands::Cancel { bet_id, execute } => {
            if execute {
                eprintln!("Executing REAL cancellation of limit order {bet_id}.");
            } else {
                eprintln!("PREVIEW ONLY: no cancellation request will be sent.");
            }
            print_json(&client.cancel_limit_order(&bet_id, execute).await?)?;
        }
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
            before,
            after,
            username,
            contract_slug,
            open_limit,
            before_time,
            after_time,
            order,
        } => {
            if limit <= 0 {
                bail!("Bet limit must be positive");
            }
            let bets = client
                .list_bets_filtered(BetFilters {
                    user_id: user_id.as_deref(),
                    username: username.as_deref(),
                    market_id: market_id.as_deref(),
                    contract_slug: contract_slug.as_deref(),
                    limit: Some(limit),
                    before: before.as_deref(),
                    after: after.as_deref(),
                    before_time,
                    after_time,
                    kinds: if open_limit { Some("open-limit") } else { None },
                    order: order.as_deref(),
                })
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
