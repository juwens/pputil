use derive_setters::Setters;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::widgets::{Padding, Tabs, Widget};
use ratatui::widgets::{Block, Borders, Clear};
use strum::IntoEnumIterator;
use std::default::Default;
use std::fmt::Debug;

use super::tui_main::{DetailTabKind, DetailTabStruct, KeyBindings};

#[derive(Debug, Setters)]
pub struct DetailsPopup<'a> {
    pub title: Line<'a>,
    pub key_bindings: KeyBindings,
    pub selected_tab_index: usize,
    pub selected_tab: DetailTabStruct<'a>,
}

impl Widget for DetailsPopup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title)
            // .style(Style::new().on_magenta())
            .padding(Padding::new(1,1,1,1))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let vertical_stack = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .spacing(0)
        .split(inner);
        let [tabs_header_area, tabs_content_area] = [vertical_stack[0], vertical_stack[1]];

        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        let titles = DetailTabKind::iter().map(DetailTabKind::title);
        Tabs::new(titles)
        .highlight_style(highlight_style)
        .select(self.selected_tab_index)
        .render(tabs_header_area, buf);

        self.selected_tab.render(tabs_content_area, buf);
    }
}
