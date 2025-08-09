use crate::args::{CompactSortBy, ListTuiArgs, SortOrder};
use crate::helpers::{encode_to_yaml_str, ProvisioningProfileFileData, UnwrapOrNa, NOT_AVAILABLE};
use crate::tui::DetailsPopup;
use crate::types::ProfilesCollection;
use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode};
use derive_setters::Setters;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::palette::tailwind;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    TableState, Widget,
};
use ratatui::{
    layout::Constraint,
    widgets::{Block, Borders, Row, Table},
};
use ratatui::{symbols, DefaultTerminal};
use std::fs::remove_file;
use std::rc::Rc;
use std::time::SystemTime;
use std::{io, vec};
use strum::{Display, EnumIter, FromRepr};

#[derive(Debug, Setters)]
pub struct TuiApp {
    profiles: Vec<Rc<ProvisioningProfileFileData>>,
    #[setters]
    selected_index: usize,
    selected_tab_index: usize,
    key_bindings: KeyBindings,
    show_details_popup: bool,
    prov_profiles_scroll_position: usize,
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter, Debug)]
pub enum DetailTabKind {
    #[default]
    #[strum(to_string = "properties")]
    Properties,
    #[strum(to_string = "prov devices")]
    ProvisionedDevices,
    #[strum(to_string = "certificates")]
    DeveloperCertificates,
}

#[derive(Debug)]
pub struct DetailTabStruct<'a> {
    profile: &'a ProvisioningProfileFileData,
    kind: DetailTabKind,
    pub vertical_scroll_position: usize,
    pub vertical_scroll_state: ScrollbarState,
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub quit: KeyCode,
    pub profile_scroll_up_one: KeyCode,
    pub profile_scroll_down_one: KeyCode,
    pub profile_delete: KeyCode,
    pub tab_properties: KeyCode,
    pub tab_provisioning_devices: KeyCode,
    pub tab_developer_certificates: KeyCode,
    pub open_details: KeyCode,
    pub profile_scroll_up_page: KeyCode,
    pub profile_scroll_down_page: KeyCode,
    pub profile_scroll_top: KeyCode,
    pub profile_scroll_bottom: KeyCode,
}

impl TuiApp {
    pub fn new(
        profiles_unsorted: ProfilesCollection,
        args: &ListTuiArgs,
    ) -> TuiApp {
        let mut profiles = profiles_unsorted
            .iter()
            .map(|row| match row {
                Err(x) | Ok(x) => x.clone(),
            })
            .collect_vec();

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

        let key_bindings = KeyBindings {
            quit: KeyCode::Char('q'),
            profile_scroll_up_one: KeyCode::Up,
            profile_scroll_down_one: KeyCode::Down,
            profile_scroll_up_page: KeyCode::PageUp,
            profile_scroll_down_page: KeyCode::PageDown,
            profile_scroll_top: KeyCode::Char('g'),
            profile_scroll_bottom: KeyCode::Char('G'),
            profile_delete: KeyCode::Char('d'),
            tab_properties: KeyCode::Char('1'),
            tab_provisioning_devices: KeyCode::Char('2'),
            tab_developer_certificates: KeyCode::Char('3'),
            open_details: KeyCode::Enter,
        };

        Self {
            profiles,
            selected_index: 0,
            selected_tab_index: 0,
            key_bindings,
            show_details_popup: false,
            prov_profiles_scroll_position: 0,
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
                if key.code == self.key_bindings.quit || key.code == KeyCode::Esc {
                    if self.show_details_popup {
                        self.show_details_popup = false;
                    } else {
                        return Ok(());
                    }
                } else if key.code == self.key_bindings.tab_properties {
                    self.selected_tab_index = 0;
                } else if key.code == self.key_bindings.tab_provisioning_devices {
                    self.selected_tab_index = 1;
                } else if key.code == self.key_bindings.tab_developer_certificates {
                    self.selected_tab_index = 2;
                } else if key.code == self.key_bindings.open_details {
                    self.show_details_popup = !self.show_details_popup;
                } else if key.code == self.key_bindings.profile_delete {
                    let selected_profile = &self.profiles[self.selected_index];

                    let confirmed = true;
                    if confirmed {
                        let to_delete_file_path = &selected_profile.file_path;
                        let file = to_delete_file_path.as_os_str();
                        let remove_file_res = remove_file(file);
                        if let Err(e) = remove_file_res {
                            log_error(format!("failed to delete file '{}'\n{}", file.to_string_lossy(), e))
                        } else {
                            let new_profiles = self.profiles.iter()
                                .filter(|x| to_delete_file_path.eq(&x.file_path))
                                .cloned()
                                .collect::<Vec<_>>();

                            self.profiles = new_profiles;
                        }
                    }
                }

                /* TODO: calc rows per page */
                let rows_per_page = 40;
                let total_nr_of_rows: usize = self.profiles.len() - 1;

                match key.code {
                    KeyCode::PageUp => {
                        decrement(&mut self.selected_index, rows_per_page);
                    }
                    KeyCode::PageDown => {
                        increment(&mut self.selected_index, rows_per_page, total_nr_of_rows);
                    }
                    KeyCode::Up => {
                        decrement(&mut self.selected_index, 1);
                    }
                    KeyCode::Down => {
                        increment(&mut self.selected_index, 1, total_nr_of_rows);
                    }
                    KeyCode::Char('j') => {
                        self.prov_profiles_scroll_position =
                            self.prov_profiles_scroll_position.saturating_add(10);
                    }
                    KeyCode::Char('k') => {
                        self.prov_profiles_scroll_position =
                            self.prov_profiles_scroll_position.saturating_sub(10);
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
                    profile.provisioned_devices.len().to_string(),
                    // profile.uuid.unwrap_or_na(),
                    // profile.xc_kind.unwrap_or_na(),
                ];

                Row::new(cells)
            })
            .collect();

        #[allow(clippy::cast_possible_truncation)]
        let index_width: u16 = format!("{}", rows.len()).len() as u16;

        let widths = [
            Constraint::Length(index_width),
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

fn log_error(format: String) {
    panic!("{}", format)
}

impl Widget for &mut TuiApp {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let popup_area = Rect {
            x: area.width * 5 / 100,
            y: area.height * 5 / 100,
            width: area.width * 9 / 10,
            height: area.height * 9 / 10,
        };

        let key_bindings = [
            format!("({}) quit", self.key_bindings.quit),
            format!("({}) move up", self.key_bindings.profile_scroll_up_one), // ↑
            format!("({}) move down", self.key_bindings.profile_scroll_down_one), // ↓
            format!("({}) delete profile", self.key_bindings.profile_delete),
        ];

        let surrounding_block = Block::bordered()
            .title(" pputil ")
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(format!(" {} ", key_bindings.join(" | ")));

        let mut table_state = TableState::default().with_selected(self.selected_index);

        StatefulWidget::render(
            self.create_table().block(surrounding_block),
            area,
            buf,
            &mut table_state,
        );

        let selected_profile: &ProvisioningProfileFileData =
            self.profiles.get(self.selected_index).unwrap();
        let selected_tab = DetailTabStruct {
            profile: selected_profile,
            kind: match self.selected_tab_index {
                0 => DetailTabKind::Properties,
                1 => DetailTabKind::ProvisionedDevices,
                2 => DetailTabKind::DeveloperCertificates,
                _ => todo!(),
            },
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll_position: self.prov_profiles_scroll_position,
        };

        if self.show_details_popup {
            let popup = DetailsPopup {
                selected_tab_index: self.selected_tab_index,
                key_bindings: self.key_bindings.clone(),
                title: Line::from("Details"),
                selected_tab,
            };
            popup.render(popup_area, buf);
        }
    }
}

pub fn run_tui_mode(
    profiles_unsorted: ProfilesCollection,
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

impl Widget for DetailTabStruct<'_> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // in a real app these might be separate widgets
        match self.kind {
            DetailTabKind::Properties => self.render_tab_properties(area, buf, self.profile),
            DetailTabKind::ProvisionedDevices => {
                self.render_tab_provisioning_devices(area, buf, self.profile);
            }
            DetailTabKind::DeveloperCertificates => {
                self.render_tab_developer_certificates(area, buf, self.profile);
            }
        }
    }
}

impl DetailTabKind {
    pub fn title(self) -> Line<'static> {
        let text = match self {
            DetailTabKind::Properties => "Properties",
            DetailTabKind::ProvisionedDevices => "ProvisionedDevices",
            DetailTabKind::DeveloperCertificates => "DeveloperCertificates",
        };
        format!("  {text}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    /// A block surrounding the tab's content
    fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    pub const fn palette(self) -> tailwind::Palette {
        match self {
            DetailTabKind::Properties => tailwind::RED,
            DetailTabKind::ProvisionedDevices => tailwind::LIME,
            DetailTabKind::DeveloperCertificates => tailwind::SKY,
        }
    }
}

impl DetailTabStruct<'_> {
    fn block(&self) -> Block<'static> {
        self.kind.block()
    }
    pub const fn palette(&self) -> tailwind::Palette {
        self.kind.palette()
    }

    fn render_tab_properties(
        &self,
        area: Rect,
        buf: &mut Buffer,
        profile: &ProvisioningProfileFileData,
    ) {
        Paragraph::new(Text::raw(encode_to_yaml_str(&profile.properties)))
            .block(self.block())
            .render(area, buf);
    }

    fn render_tab_provisioning_devices(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        profile: &ProvisioningProfileFileData,
    ) {
        let mut sorted = profile.provisioned_devices.clone();
        sorted.sort();
        let sorted = sorted;

        let rows: Vec<Row> = sorted
            .iter()
            .enumerate()
            .map(|(i, x)| Row::new(vec![i.to_string(), x.to_string()]))
            .collect();
        let widths = vec![Constraint::Length(2), Constraint::Fill(1)];
        let mut tbl_state = TableState::new().with_offset(self.vertical_scroll_position);
        let tbl = Table::new(rows.clone(), widths)
            .header(Row::new(vec!["nr", "uuid"]))
            .block(self.block());

        ratatui::widgets::StatefulWidget::render(&tbl, area, buf, &mut tbl_state);

        let scoll_bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        self.vertical_scroll_state = self.vertical_scroll_state.content_length(rows.len());
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(self.vertical_scroll_position);

        ratatui::widgets::StatefulWidget::render(
            scoll_bar,
            area,
            buf,
            &mut self.vertical_scroll_state,
        );
    }

    fn render_tab_developer_certificates(
        &self,
        area: Rect,
        buf: &mut Buffer,
        profile: &ProvisioningProfileFileData,
    ) {
        let headers = Row::new(vec!["subject", ""]);
        let widths = vec![Constraint::Fill(10), Constraint::Fill(1)];
        let rows = profile
            .developer_certificates
            .iter()
            .map(|x| Row::new(vec![Text::from(x.subject.to_string()), Text::from("")]))
            .collect::<Vec<_>>();
        let tbl = Table::new(rows, widths).header(headers);

        Widget::render(tbl, area, buf);

        // let list = profile
        //     .developer_certificates
        //     .iter()
        //     // .map(|cert| {
        //     //     Paragraph::new(vec![
        //     //         Line::raw(format!("subject: {}", cert.subject)),
        //     //         Line::raw(format!("issuer: {}", cert.issuer)),
        //     //     ])
        //     // })
        //     .flat_map(|x| { vec![
        //         ListItem::new(format!("subject: {}", x.subject.clone())),
        //         // ListItem::new(format!("issuere: {}", x.issuer.clone())),
        //         // ListItem::new("-----")
        //     ]
        //     })
        //     .collect::<List>();

        // Widget::render(list, area, buf);
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

fn decrement(field: &mut usize, by: usize) {
    *field = Ord::max(field.saturating_sub(by), 0);
}

fn increment(field: &mut usize, by: usize, max: usize) {
    *field = Ord::min(field.saturating_add(by), max);
}
