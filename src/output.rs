use comfy_table::{
    Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
};
use miette::{Result, miette};
use precious_core::cost::{Breakdown, Change, Diff};
use serde::Serialize;

pub fn print_breakdown_table(breakdown: &Breakdown) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Resource"),
        Cell::new("Cost Component"),
        Cell::new("Quantity"),
        Cell::new("Unit Price"),
        Cell::new("Monthly Cost"),
    ]);

    for resource in &breakdown.resources {
        let mut first = true;
        for component in &resource.components {
            let resource_cell = if first {
                first = false;
                Cell::new(format!(
                    "{}\n  {}",
                    resource.address, resource.resource_type
                ))
            } else {
                Cell::new("")
            };

            let quantity_str = match component.quantity_max {
                Some(max) => format!(
                    "{}–{} {}",
                    component.quantity, max, component.quantity_unit
                ),
                None => format!("{} {}", component.quantity, component.quantity_unit),
            };

            let cost_str = match component.monthly_cost_max {
                Some(max) => format!("{}–{}", component.monthly_cost, max),
                None => component.monthly_cost.to_string(),
            };

            table.add_row(vec![
                resource_cell,
                Cell::new(&*component.name),
                Cell::new(quantity_str),
                Cell::new(component.unit_price.to_string()),
                Cell::new(cost_str),
            ]);
        }
    }

    let total_str = match breakdown.total_monthly_cost_max {
        Some(max) => format!("{}–{}", breakdown.total_monthly_cost, max),
        None => breakdown.total_monthly_cost.to_string(),
    };

    table.add_row(vec![
        Cell::new("TOTAL").fg(Color::Green),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
        Cell::new(total_str).fg(Color::Green),
    ]);

    println!("\n{table}\n");
}

pub fn print_diff_table(diff: &Diff) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Change"),
        Cell::new("Resource"),
        Cell::new("Before"),
        Cell::new("After"),
        Cell::new("Delta"),
    ]);

    for change in &diff.changes {
        match change {
            Change::Added(cost) => {
                table.add_row(vec![
                    Cell::new("+").fg(Color::Green),
                    Cell::new(cost.address.to_string()),
                    Cell::new("-"),
                    Cell::new(cost.monthly_total.to_string()),
                    Cell::new(format!("+{}", cost.monthly_total)).fg(Color::Green),
                ]);
            }
            Change::Removed(cost) => {
                table.add_row(vec![
                    Cell::new("-").fg(Color::Red),
                    Cell::new(cost.address.to_string()),
                    Cell::new(cost.monthly_total.to_string()),
                    Cell::new("-"),
                    Cell::new(format!("-{}", cost.monthly_total)).fg(Color::Red),
                ]);
            }
            Change::Modified { before, after } => {
                let delta = after.monthly_total - before.monthly_total;
                let color = if delta.amount.is_sign_positive() {
                    Color::Red
                } else {
                    Color::Green
                };
                table.add_row(vec![
                    Cell::new("~").fg(Color::Yellow),
                    Cell::new(after.address.to_string()),
                    Cell::new(before.monthly_total.to_string()),
                    Cell::new(after.monthly_total.to_string()),
                    Cell::new(delta.to_string()).fg(color),
                ]);
            }
        }
    }

    println!("\n{table}");
    println!(
        "\nTotal before: {}  |  Total after: {}  |  Delta: {}\n",
        diff.total_before, diff.total_after, diff.delta
    );
}

pub fn print_json<T: Serialize>(data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data).map_err(|e| miette!("JSON error: {e}"))?;
    println!("{json}");
    Ok(())
}
