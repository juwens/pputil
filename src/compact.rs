use args::{Cli, CompactSortBy};
use chrono::{DateTime, Local};
use comfy_table::{Cell, Row};
use std::vec;

use crate::args::Commands;
use crate::helpers::{IntoCell, ToStringExt, UnwrapOrNa, NOT_AVAILABLE};
use crate::{args, PrivisionFileData};

pub fn print_compact_table(iter: impl Iterator<Item = PrivisionFileData>, args: &Cli) {
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
        Commands::PrintCompact(compact_args) => {
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

    let wrap = match &args.command.as_ref().unwrap() {
        Commands::PrintCompact(compact_args) => compact_args.allow_wrap,
    };

    for file_data in rows {
        let mut table_row: Row = Row::new();

        if !wrap {
            table_row.max_height(1);
        }

        let mut add = |x: Cell| {
            table_row.add_cell(x);
        };

        add(file_data.name.unwrap_or_na().into_cell());
        add(file_data.app_id_name.unwrap_or_na().into_cell());

        add(file_data.ent_app_id.unwrap_or_na().into_cell());

        add(file_data.exp_date.map(DateTime::<Local>::from).map_or_else(
            || Cell::new(NOT_AVAILABLE),
            |x| {
                let s = x.format("%Y-%m-%d").to_string();
                let c = Cell::new(s);
                if x.le(&chrono::Utc::now()) {
                    return c.fg(comfy_table::Color::Red);
                }
                c
            },
        ));

        add(file_data
            .xc_managed
            .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            .to_string()
            .into_cell());

        add(file_data
            .local_provision
            .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            .to_string()
            .into_cell());

        add(file_data.team_name.unwrap_or_na().into_cell());

        add(file_data
            .provisioned_devices
            .map_or(String::from(NOT_AVAILABLE), |x| x.to_string())
            .into_cell());

        add(file_data.uuid.unwrap_or_na().into_cell());

        table.add_row(table_row);
    }

    println!("{table}");
}
