use chrono::{DateTime, Local};
use comfy_table::{Cell, Row};
use std::vec;

use crate::args;
use crate::args::{CompactSortBy, ListCompactArgs};
use crate::helpers::{IntoCell, ProvisioningProfileFileData, UnwrapOrNa, NOT_AVAILABLE};

pub fn print_compact_table(
    profiles_unsorted: impl Iterator<
        Item = Result<ProvisioningProfileFileData, ProvisioningProfileFileData>,
    >,
    args: &ListCompactArgs,
) {
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
        "XC",
    ]);

    let mut profiles_sorted = profiles_unsorted
        // Result.Err is a profile which failed to parse for some reason and contains dummy profile-data
        .map(|row| match row {
            Err(x) | Ok(x) => x,
        })
        .collect::<Vec<_>>();

    match args.sort_by {
        CompactSortBy::Name => {
            profiles_sorted.sort_by_key(|x| x.name.unwrap_or_na().to_lowercase());
        }
        CompactSortBy::AppIdName => {
            profiles_sorted.sort_by_key(|x| x.app_id_name.unwrap_or_na().to_lowercase());
        }
        CompactSortBy::ExpirationDate => {
            profiles_sorted
                .sort_by_key(|x| x.exp_date);
        }
    };

    if args.sort_order == args::SortOrder::Desc {
        profiles_sorted.reverse();
    }

    let profiles_sorted = profiles_sorted;

    for profile in profiles_sorted {
        let mut table_row: Row = Row::new();

        if !args.allow_wrap {
            table_row.max_height(1);
        }

        let mut add = |x: Cell| {
            table_row.add_cell(x);
        };

        add(profile.name.unwrap_or_na().into_cell());
        add(profile.app_id_name.unwrap_or_na().into_cell());

        add(profile.ent_app_id.unwrap_or_na().into_cell());

        add(profile.exp_date.map(DateTime::<Local>::from).map_or_else(
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

        add(profile
            .xc_managed
            .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            .to_string()
            .into_cell());

        add(profile
            .local_provision
            .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            .to_string()
            .into_cell());

        add(profile.team_name.unwrap_or_na().into_cell());

        add(profile
            .provisioned_devices
            .map_or(String::from(NOT_AVAILABLE), |x| x.to_string())
            .into_cell());

        add(profile.uuid.unwrap_or_na().into_cell());

        add(profile.xc_kind.unwrap_or_na().into_cell());

        table.add_row(table_row);
    }

    println!("{table}");
}
