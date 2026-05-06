use comfy_table::{Attribute, Cell, Color, Table, ContentArrangement};
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

pub fn print_positions_table(positions: &[ContractMetric]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new("Market ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Invested").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Profit %").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("User").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    let mut max_pos = 0.01; // Avoid div by zero
    let mut min_neg = -0.01;

    for p in positions {
        if p.profit > max_pos { max_pos = p.profit; }
        if p.profit < min_neg { min_neg = p.profit; }
    }

    let mut total_invested = 0.0;
    let mut total_profit = 0.0;

    for p in positions {
        total_invested += p.invested;
        total_profit += p.profit;

        let color = if p.profit > 0.0 {
            let ratio = (p.profit / max_pos).min(1.0).powf(0.6);
            Color::Rgb { 
                r: (200.0 * (1.0 - ratio)) as u8, 
                g: (200.0 + 55.0 * ratio) as u8, 
                b: (200.0 * (1.0 - ratio)) as u8 
            }
        } else if p.profit < 0.0 {
            let ratio = (p.profit / min_neg).min(1.0).powf(0.6);
            Color::Rgb { 
                r: (200.0 + 55.0 * ratio) as u8, 
                g: (200.0 * (1.0 - ratio)) as u8, 
                b: (200.0 * (1.0 - ratio)) as u8 
            }
        } else {
            Color::Reset
        };

        table.add_row(vec![
            Cell::new(&p.contract_id),
            Cell::new(&format!("{:.2}", p.invested)),
            Cell::new(&format!("{:.2}", p.profit)).fg(color),
            Cell::new(&format!("{:.2}%", p.profit_percent)).fg(color),
            Cell::new(p.user_username.as_deref().unwrap_or("N/A")),
        ]);
    }

    let total_color = if total_profit > 0.0 { Color::Green } else if total_profit < 0.0 { Color::Red } else { Color::Reset };

    table.add_row(vec![
        Cell::new("TOTAL").add_attribute(Attribute::Bold),
        Cell::new(&format!("{:.2}", total_invested)).add_attribute(Attribute::Bold).fg(Color::Yellow),
        Cell::new(&format!("{:.2}", total_profit)).add_attribute(Attribute::Bold).fg(total_color),
        Cell::new(&format!("{:.2}%", (total_profit / total_invested.max(1.0)) * 100.0)).add_attribute(Attribute::Bold).fg(total_color),
        Cell::new(""),
    ]);

    println!("{}", table);
}
