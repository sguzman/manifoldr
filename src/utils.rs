use comfy_table::{Attribute, Cell, Color, Table, ContentArrangement};
use comfy_table::presets::UTF8_FULL;
use crate::api::models::*;

pub fn print_user_table(user: &User) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    
    table.add_row(vec![
        Cell::new("Field").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Value").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    table.add_row(vec!["ID", &user.id]);
    table.add_row(vec!["Username", &user.username]);
    table.add_row(vec!["Name", &user.name]);
    table.add_row(vec!["Balance", &format!("{:.2}", user.balance)]);
    table.add_row(vec!["Total Deposits", &format!("{:.2}", user.total_deposits)]);
    table.add_row(vec!["Bio", user.bio.as_deref().unwrap_or("N/A")]);
    
    println!("{}", table);
}

pub fn print_markets_table(markets: &[LiteMarket]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    
    table.add_row(vec![
        Cell::new("ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Question").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Prob").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Vol").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Resolved").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    for m in markets {
        let prob = m.probability.map(|p| format!("{:.0}%", p * 100.0)).unwrap_or_else(|| "N/A".to_string());
        let resolved = if m.is_resolved {
            m.resolution.clone().unwrap_or_else(|| "YES".to_string())
        } else {
            "NO".to_string()
        };

        let volume = m.volume.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "N/A".to_string());

        table.add_row(vec![
            &m.id,
            &m.question,
            &prob,
            &volume,
            &resolved,
        ]);
    }

    println!("{}", table);
}

pub fn print_bets_table(bets: &[Bet]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new("ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Amount").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Outcome").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Market ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    for b in bets {
        table.add_row(vec![
            &b.id,
            &format!("{:.2}", b.amount),
            &b.outcome,
            &b.contract_id,
        ]);
    }

    println!("{}", table);
}
pub fn print_portfolio_history_table(history: &[PortfolioMetrics]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new("Timestamp").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Investment").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Balance").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    for h in history {
        let profit = h.profit.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "N/A".to_string());
        table.add_row(vec![
            &h.timestamp.to_string(), // Could format this better but keeping it simple
            &format!("{:.2}", h.investment_value),
            &format!("{:.2}", h.balance),
            &profit,
        ]);
    }

    println!("{}", table);
}

fn format_shares(shares: &std::collections::HashMap<String, Option<f64>>) -> String {
    let mut parts = Vec::new();
    let mut sorted_keys: Vec<_> = shares.keys().collect();
    sorted_keys.sort();
    
    for outcome in sorted_keys {
        if let Some(Some(amt)) = shares.get(outcome) {
            if amt.abs() > 0.1 {
                parts.push(format!("{}: {:.0}", outcome, amt));
            }
        }
    }
    
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn print_positions_table(
    position_groups: &[Vec<ContractMetric>],
    titles: Option<&std::collections::HashMap<String, String>>,
    max_width: Option<u16>,
    display_limit: Option<usize>,
    all: bool,
) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let filtered_groups: Vec<Vec<ContractMetric>> = if all {
        position_groups.to_vec()
    } else {
        position_groups.iter()
            .map(|group| {
                group.iter().filter(|p| {
                    let total_shares: f64 = p.total_shares.values().map(|v| v.unwrap_or_default()).sum();
                    total_shares.abs() > 0.1
                })
                .cloned()
                .collect::<Vec<_>>()
            })
            .filter(|group| !group.is_empty())
            .collect()
    };

    let mut headers = vec![
        Cell::new("Market").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Outcome").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Invested").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit %").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ];

    let show_user = titles.is_none();
    if show_user {
        headers.push(Cell::new("User").add_attribute(Attribute::Bold).fg(Color::Cyan));
    }

    table.add_row(headers);

    // Prioritize space for the Market column and set reasonable boundaries
    table.column_mut(0).expect("Market column missing")
        .set_constraint(comfy_table::ColumnConstraint::LowerBoundary(comfy_table::Width::Fixed(30)));
    
    // Numerical columns should be tight
    for i in 2..5 {
        table.column_mut(i).expect("Column missing")
            .set_constraint(comfy_table::ColumnConstraint::ContentWidth);
    }

    let mut max_pos = 0.01;
    let mut min_neg = -0.01;

    for group in &filtered_groups {
        for p in group {
            for (outcome, value) in &p.total_shares {
                let val = value.unwrap_or_default();
                if val.abs() < 0.1 { continue; }
                let invested = p.total_spent.get(outcome).unwrap_or(&None).unwrap_or_default();
                let profit = val - invested;
                if profit > max_pos { max_pos = profit; }
                if profit < min_neg { min_neg = profit; }
            }
        }
    }

    let mut total_invested = 0.0;
    let mut total_profit = 0.0;

    let display_groups = if let Some(limit) = display_limit {
        filtered_groups.iter().take(limit).collect::<Vec<_>>()
    } else {
        filtered_groups.iter().collect::<Vec<_>>()
    };

    // Calculate totals from ALL positions
    for group in position_groups {
        for p in group {
            total_invested += p.invested.unwrap_or_default();
            total_profit += p.profit.unwrap_or_default();
        }
    }

    for group in display_groups {
        let mut row_in_group = 0;
        
        // Collect all individual positions from all metrics in the group
        let mut individual_positions = Vec::new();
        for p in group {
            let mut sorted_outcomes: Vec<_> = p.total_shares.keys().collect();
            sorted_outcomes.sort();
            
            for outcome in sorted_outcomes {
                let value = p.total_shares.get(outcome).unwrap_or(&None).unwrap_or_default();
                if value.abs() < 0.1 { continue; }
                
                let invested = p.total_spent.get(outcome).unwrap_or(&None).unwrap_or_default();
                let profit = value - invested;
                let profit_pct = if invested > 0.1 { (profit / invested) * 100.0 } else { 0.0 };
                
                individual_positions.push((outcome, value, invested, profit, profit_pct, p));
            }
        }

        for (outcome, _value, invested, profit, profit_pct, p) in individual_positions {
            let color = if profit > 0.0 {
                let ratio = (profit / max_pos).min(1.0).powf(0.6);
                Color::Rgb { 
                    r: (200.0 * (1.0 - ratio)) as u8, 
                    g: (200.0 + 55.0 * ratio) as u8, 
                    b: (200.0 * (1.0 - ratio)) as u8 
                }
            } else if profit < 0.0 {
                let ratio = (profit / min_neg).min(1.0).powf(0.6);
                Color::Rgb { 
                    r: (200.0 + 55.0 * ratio) as u8, 
                    g: (200.0 * (1.0 - ratio)) as u8, 
                    b: (200.0 * (1.0 - ratio)) as u8 
                }
            } else {
                Color::Reset
            };

            let mut market_display = if row_in_group == 0 {
                titles
                    .and_then(|m| m.get(&p.contract_id))
                    .cloned()
                    .unwrap_or_else(|| p.contract_id.clone())
            } else {
                "".to_string()
            };

            if let Some(limit) = max_width {
                let limit = limit as usize;
                if market_display.chars().count() > limit {
                    market_display = market_display.chars().take(limit - 3).collect::<String>() + "...";
                }
            }

            let mut row = vec![
                Cell::new(market_display),
                Cell::new(outcome), // Just the outcome name (e.g. YES, NO, or Answer ID)
                Cell::new(&format!("{:.2}", invested)),
                Cell::new(&format!("{:.2}", profit)).fg(color),
                Cell::new(&format!("{:.2}%", profit_pct)).fg(color),
            ];

            if show_user {
                let user_display = if row_in_group == 0 {
                    p.user_username.as_deref().unwrap_or(&p.user_id)
                } else {
                    ""
                };
                row.push(Cell::new(user_display));
            }

            table.add_row(row);
            row_in_group += 1;
        }
    }

    let total_color = if total_profit > 0.0 { Color::Green } else if total_profit < 0.0 { Color::Red } else { Color::Reset };

    let mut footer = vec![
        Cell::new("TOTAL").add_attribute(Attribute::Bold),
        Cell::new(""),
        Cell::new(&format!("{:.2}", total_invested)).add_attribute(Attribute::Bold).fg(Color::Yellow),
        Cell::new(&format!("{:.2}", total_profit)).add_attribute(Attribute::Bold).fg(total_color),
        Cell::new(&format!("{:.2}%", (total_profit / total_invested.max(1.0)) * 100.0)).add_attribute(Attribute::Bold).fg(total_color),
    ];

    if show_user {
        footer.push(Cell::new(""));
    }

    table.add_row(footer);

    println!("{}", table);
}
