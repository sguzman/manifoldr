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

        table.add_row(vec![
            &m.id,
            &m.question,
            &prob,
            &format!("{:.0}", m.volume),
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
