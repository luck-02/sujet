use ratatui::buffer::Buffer;
use ratatui::layout::{Layout, Rect};
use ratatui::layout::Constraint::Ratio;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, Gauge, Padding, Paragraph, Widget};

pub fn split_horizontal<const N: usize>(area: Rect) -> [Rect; N] {
    Layout::horizontal([Ratio(1, N as u32); N]).areas(area)
}

pub fn split_vertical<const N: usize>(area: Rect) -> [Rect; N] {
    Layout::vertical([Ratio(1, N as u32); N]).areas(area)
}

fn title_block(title: &str) -> Block {
    let title = Line::from(title).centered();
    Block::new()
        .borders(Borders::NONE)
        .padding(Padding::vertical(1))
        .title(title)
}

pub fn progress_bar(area: Rect, buffer: &mut Buffer, title: &str, progress: f64, total: f64) {
    let title = title_block(title);
    Gauge::default()
        .block(title)
        .percent(((progress / total) * 100.0).round() as u16)
        .render(area, buffer);
}

pub fn text<T: ToString>(area: Rect, buffer: &mut Buffer, val: T) {
    let block = Block::bordered();
    let text = Text::from(Line::from(val.to_string()));
    Paragraph::new(text).block(block).render(area, buffer)
}

pub fn vertical_barchart(area: Rect, buffer: &mut Buffer, title: &str, data: &[(String, f64)]) {
    let w = area.width / (data.len() as u16);
    let bars: Vec<Bar> = data
        .iter()
        .map(|(label, value)| vertical_bar(label, value))
        .collect();

    let title = Line::from(title).centered();
    BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .block(Block::new().title(title))
        .bar_width(w-1)
        .render(area, buffer)
}

fn vertical_bar<'a>(label: &'a str, data: &'a f64) -> Bar<'a> {
    let n = (*data * 1000.0) as u64;
    Bar::default()
        .value(n)
        .label(Line::from(label))
        .text_value(format!("{data:>3}"))
}
