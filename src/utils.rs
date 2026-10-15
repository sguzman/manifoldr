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

fn abbreviate(value: &str, max_width: Option<u16>) -> String {
    let Some(width) = max_width.map(usize::from) else {
        return value.to_owned();
    };
    if value.chars().count() <= width {
        return value.to_owned();
    }
    if width <= 3 {
        return value.chars().take(width).collect();
    }
    format!("{}...", value.chars().take(width - 3).collect::<String>())
}

fn describe_shares(shares: &std::collections::HashMap<String, Option<f64>>) -> String {
    let mut positions: Vec<_> = shares
        .iter()
        .filter_map(|(outcome, value)| match value {
            Some(n) if n.is_finite() && n.abs() > 0.000001 => {
                Some(format!("{outcome}: {n:.2}"))
            }
            _ => None,
        })
        .collect();
    positions.sort();
    if positions.is_empty() {
        "-".to_owned()
    } else {
        positions.join(", ")
    }
}

fn money(value: Option<f64>) -> String {
    value.map_or_else(|| "N/A".to_owned(), |number| format!("{number:.2}"))
}

fn profit_cell(value: Option<f64>, percent: bool) -> Cell {
    let content = if percent {
        value.map_or_else(|| "N/A".to_owned(), |n| format!("{n:.2}%"))
    } else {
        money(value)
    };
    let color = match value {
        Some(n) if n > 0.0 => Color::Green,
        Some(n) if n < 0.0 => Color::Red,
        _ => Color::Reset,
    };
    Cell::new(content).fg(color)
}

/// Values and P/L always come from Manifold's ContractMetric fields.
/// Raw share counts are quantities, NOT currently redeemable payouts.
pub fn print_positions_table(
    position_groups: &[Vec<ContractMetric>],
    titles: Option<&std::collections::HashMap<String, String>>,
    max_width: Option<u16>,
    display_limit: Option<usize>,
    all: bool,
) {
    let filtered: Vec<&Vec<ContractMetric>> = position_groups
        .iter()
        .filter(|group| {
            all || group.iter().any(|p| {
                p.total_shares.values().any(|v| v.is_some_and(|n| n.abs() > 0.000001))
            })
        })
        .collect();
    let visible: Vec<_> = filtered
        .iter()
        .take(display_limit.unwrap_or(usize::MAX))
        .copied()
        .collect();

    let show_user = titles.is_none();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    let mut header = vec![
        Cell::new("Market").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Shares").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Invested").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit %").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ];
    if show_user {
        header.push(Cell::new("User").add_attribute(Attribute::Bold).fg(Color::Cyan));
    }
    table.add_row(header);
    table.column_mut(0).expect("Market column").set_constraint(
        comfy_table::ColumnConstraint::LowerBoundary(comfy_table::Width::Fixed(30)),
    );

    for group in &visible {
        for (index, p) in group.iter().enumerate() {
            let market = if index == 0 {
                titles.and_then(|m| m.get(&p.contract_id))
                    .map(String::as_str)
                    .unwrap_or(&p.contract_id)
            } else {
                ""
            };
            let mut row = vec![
                Cell::new(abbreviate(market, max_width)),
                Cell::new(describe_shares(&p.total_shares)),
                Cell::new(money(p.invested)),
                profit_cell(p.profit, false),
                profit_cell(p.profit_percent, true),
            ];
            if show_user {
                row.push(Cell::new(if index == 0 {
                    p.user_username.as_deref().unwrap_or(&p.user_id)
                } else {
                    ""
                }));
            }
            table.add_row(row);
        }
    }

    // Both totals are sums of the API's position-level invested/profit fields.
    let totals = |groups: &[&Vec<ContractMetric>]| -> (f64, f64, usize) {
        groups.iter().flat_map(|g| g.iter()).fold(
            (0.0, 0.0, 0),
            |(invested, profit, missing), p| (
                invested + p.invested.unwrap_or_default(),
                profit + p.profit.unwrap_or_default(),
                missing + usize::from(p.invested.is_none() || p.profit.is_none()),
            ),
        )
    };
    let mut add_total = |label: &str, groups: &[&Vec<ContractMetric>]| {
        let (invested, profit, missing) = totals(groups);
        let mut row = vec![
            Cell::new(label).add_attribute(Attribute::Bold),
            Cell::new(""),
            Cell::new(format!("{invested:.2}")).add_attribute(Attribute::Bold),
            profit_cell(if missing == 0 { Some(profit) } else { None }, false),
            profit_cell(
                if missing == 0 && invested > 0.0 {
                    Some(100.0 * profit / invested)
                } else {
                    None
                },
                true,
            ),
        ];
        if show_user {
            row.push(Cell::new(""));
        }
        table.add_row(row);
    };
    add_total("DISPLAYED TOTAL", &visible);
    // Include closed positions in the full total only when --all is supplied.
    add_total("TOTAL", &filtered);

    println!("{table}");
}

#[cfg(test)]
mod display_tests {
    use super::abbreviate;

    #[test]
    fn abbreviate_handles_tiny_widths_and_utf8() {
        assert_eq!(abbreviate("abcdef", Some(0)), "");
        assert_eq!(abbreviate("abcdef", Some(2)), "ab");
        assert_eq!(abbreviate("abcdef", Some(5)), "ab...");
        assert_eq!(abbreviate("áéíóú", Some(4)), "á...");
        assert_eq!(abbreviate("test", None), "test");
    }
}
