use crate::args::{CompactSortBy, ListTuiArgs, SortOrder};
use crate::helpers::{
    abbreviate_home_box, encode_to_yaml_str, ProvisioningProfileFileData, UnwrapOrNa, NOT_AVAILABLE,
};
use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::{Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Padding, Paragraph, StatefulWidget, TableState, Tabs, Widget};
use ratatui::DefaultTerminal;
use ratatui::{
    layout::Constraint,
    widgets::{Block, Borders, Row, Table},
};
use std::io;
use std::time::SystemTime;

#[derive(Debug)]
pub struct TuiApp {
    profiles: Vec<ProvisioningProfileFileData>,
    selected_index: usize,
    selected_tab_index: usize,
}

#[derive(Debug)]
enum DetailTab<'a> {
    Properties(&'a ProvisioningProfileFileData),
    ProvisionedDevices,
    DeveloperCertificates,
    None,
}

impl TuiApp {
    pub fn new(
        profiles_unsorted: impl Iterator<
            Item = Result<ProvisioningProfileFileData, ProvisioningProfileFileData>,
        >,
        args: &ListTuiArgs,
    ) -> Self {
        let mut profiles: Vec<ProvisioningProfileFileData> = profiles_unsorted
            .map(|row| match row {
                Err(x) | Ok(x) => x,
            })
            .collect();

        // Sort profiles according to args
        match args.sort_by {
            CompactSortBy::Name => {
                profiles.sort_by_key(|x| x.name.unwrap_or_na().to_lowercase());
            }
            CompactSortBy::AppIdName => {
                profiles.sort_by_key(|x| x.app_id_name.unwrap_or_na().to_lowercase());
            }
            CompactSortBy::ExpirationDate => {
                profiles.sort_by_key(|x| x.exp_date);
            }
        }

        if args.sort_order == SortOrder::Desc {
            profiles.reverse();
        }

        Self {
            profiles,
            selected_index: 0,
            selected_tab_index: 0,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let res = self.run_loop();
        ratatui::restore();
        res
    }

    fn run_loop(&mut self) -> io::Result<()> {
        let mut terminal: DefaultTerminal = ratatui::init();

        loop {
            terminal.draw(|frame| frame.render_widget(&mut *self, frame.area()))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_index < self.profiles.len().saturating_sub(1) {
                            self.selected_index += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn create_table(&self) -> Table {
        let header = Row::new(vec![
            "#",
            "Profile Name",
            "App ID Name",
            "Entitlements:\napplication-identifier",
            "expir.\ndate",
            "XC\nmgd",
            "lcl\nprv",
            "team name",
            "prv\ndvc",
        ]);

        let rows: Vec<Row> = self
            .profiles
            .iter()
            .enumerate()
            .map(|(index, profile)| {
                let cells = vec![
                    index.to_string(),
                    profile.name.unwrap_or_na(),
                    profile.app_id_name.unwrap_or_na(),
                    profile.ent_app_id.unwrap_or_na(),
                    format_expiration_date(profile.exp_date),
                    profile.xc_managed.to_tui_string(),
                    profile.local_provision.to_tui_string(),
                    profile.team_name.unwrap_or_na(),
                    profile
                        .provisioned_devices
                        .map_or(NOT_AVAILABLE.to_string(), |x| x.to_string()),
                    // profile.uuid.unwrap_or_na(),
                    // profile.xc_kind.unwrap_or_na(),
                ];

                Row::new(cells)
            })
            .collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(15),
            Constraint::Length(8),
            // Constraint::Length(8),
        ];

        Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL))
            .highlight_symbol(">> ")
            .row_highlight_style(Style::new().reversed())
    }
}

impl Widget for &mut TuiApp {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let surrounding_block = Block::bordered()
            .title(" pputil ")
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(" (q) quit | (↑) move up | (↓) move down | (d) delete profile ");

        let vertical_stack = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(5),
        ]);

        let [table_area, tabs_area, detail_area] = vertical_stack.areas(area);

        let mut table_state = TableState::default().with_selected(self.selected_index);
        // frame.render_stateful_widget(
        //     self.create_table().block(surrounding_block),
        //     table_area,
        //     &mut table_state,
        // );

        StatefulWidget::render(self.create_table()
            .block(surrounding_block), table_area, buf, &mut table_state);

        let selected_profile: &ProvisioningProfileFileData =
            self.profiles.get(self.selected_index).unwrap();
        let selected_tab = DetailTab::Properties(selected_profile);

        Tabs::new(vec!["properties", "prov devices", "certificate"])
            .select(self.selected_tab_index)
            .render(tabs_area, buf);

        selected_tab.render(detail_area, buf);
    }
}

pub fn run_tui_mode(
    profiles_unsorted: impl Iterator<
        Item = Result<ProvisioningProfileFileData, ProvisioningProfileFileData>,
    >,
    args: &ListTuiArgs,
) -> io::Result<()> {
    let mut app = TuiApp::new(profiles_unsorted, args);
    app.run()
}

trait ToTuiString {
    fn to_tui_string(&self) -> String;
}

impl ToTuiString for Option<bool> {
    fn to_tui_string(&self) -> String {
        self.map_or(NOT_AVAILABLE.to_string(), |x| {
            if x {
                "Y".to_string()
            } else {
                "N".to_string()
            }
        })
    }
}

impl Widget for DetailTab<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // in a real app these might be separate widgets
        match self {
            Self::Properties(data) => self.render_tab_properties(area, buf, data),
            Self::ProvisionedDevices => self.render_tab_provisioning_devices(area, buf),
            Self::DeveloperCertificates => self.render_tab_developer_certificates(area, buf),
            Self::None => (),
        }
    }
}

impl DetailTab<'_> {
    fn render_tab_properties(
        &self,
        area: Rect,
        buf: &mut Buffer,
        profile: &ProvisioningProfileFileData,
    ) {
        let relative_file_path = abbreviate_home_box(profile.file_path.clone());

        // let bottom_area_title = "properties";
        let bottom_area_title = format!("file: {}", relative_file_path.to_string_lossy());

        Text::raw(encode_to_yaml_str(&profile.properties)).render(area, buf);
    }

    fn render_tab_provisioning_devices(&self, area: Rect, buf: &mut Buffer) {
        todo!()
    }

    fn render_tab_developer_certificates(&self, area: Rect, buf: &mut Buffer) {
        todo!()
    }
}

fn format_expiration_date(date: Option<SystemTime>) -> String {
    date.map(DateTime::<Local>::from).map_or_else(
        || NOT_AVAILABLE.to_string(),
        |x| {
            let s = x.format("%Y-%m-%d").to_string();
            if x.le(&chrono::Utc::now()) {
                format!("{s} (EXPIRED)")
            } else {
                s
            }
        },
    )
}
