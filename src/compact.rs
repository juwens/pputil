use args::{Cli, CompactSortBy};
use chrono::{DateTime, Local};
use comfy_table::Cell;
use std::vec;

use crate::helpers::{IntoCell, ToStringExt, UnwrapOrNa, NOT_AVAILABLE};
use crate::{args, Row};

#[derive(Debug)]
enum WidthMode {
    Small,
    Unlimited,
}

pub fn print_compact_table(iter: impl Iterator<Item = Row>, args: &Cli) {
    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    table.set_header(vec![
        "Profile Name",
        "App ID Name",
        "Entitlements:\napplication-identifier",
        "expir.\ndate",
        "XC\nmgd",
        "lcl\nprv",
        "team name",
        "prv\ndvc",
        "UUID",
    ]);

    let mut rows = iter.collect::<Vec<_>>();

    match &args.command.as_ref().unwrap() {
        args::Commands::PrintCompact(compact_args) => {
            match compact_args.sort_by {
                CompactSortBy::Name => rows.sort_by_key(|x| x.name.unwrap_or_na().to_lowercase()),
                CompactSortBy::AppIdName => {
                    rows.sort_by_key(|x| x.app_id_name.unwrap_or_na().to_lowercase());
                }
                CompactSortBy::ExpirationDate => {
                    rows.sort_by_key(|x| x.exp_date.to_string().as_deref().map(str::to_lowercase));
                }
            };
            match compact_args.sort_order {
                args::SortOrder::Asc => {}
                args::SortOrder::Desc => rows.reverse(),
            }
        }
    };

    let width = match dbg!(termsize::get().unwrap().cols) {
        ..=250 => WidthMode::Small,
        _ => WidthMode::Unlimited,
    };
    dbg!(&width);

    for row in rows {
        table.add_row(vec![
            row.name.unwrap_or_na().into_cell(),
            row.app_id_name.unwrap_or_na().into_cell(),
            row.ent_app_id
                .unwrap_or_na()
                .truncate_ex(&width, 40)
                .into_cell(),
            row.exp_date.map(DateTime::<Local>::from).map_or_else(
                || Cell::new(NOT_AVAILABLE),
                |x| {
                    let s = x.format("%Y-%m-%d").to_string();
                    let c = Cell::new(s);
                    if x.le(&chrono::Utc::now()) {
                        return c.fg(comfy_table::Color::Red);
                    }
                    c
                },
            ),
            row.xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
                .to_string()
                .into_cell(),
            row.local_provision
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
                .to_string()
                .into_cell(),
            row.team_name
                .unwrap_or_na()
                .truncate_ex(&width, 12)
                .into_cell(),
            row.provisioned_devices
                .map_or(String::from(NOT_AVAILABLE), |x| x.to_string())
                .into_cell(),
            row.uuid.unwrap_or_na().truncate_ex(&width, 6).into_cell(),
        ]);
    }

    println!("{table}");
}

trait Truncate {
    fn truncate(&self, count: usize) -> Self;
    fn truncate_ex(self, mode: &WidthMode, s_len: usize) -> Self;
}

impl Truncate for String {
    fn truncate(&self, count: usize) -> String {
        if self.len() <= count {
            self.to_owned()
        } else {
            format!("{}...", &self[..count])
        }
    }

    fn truncate_ex(self, mode: &WidthMode, s_len: usize) -> Self {
        match mode {
            WidthMode::Small => self.truncate(s_len),
            WidthMode::Unlimited => self,
        }
    }
}
