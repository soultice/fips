use crate::cli::App;
use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw<B: Backend>(
    f: &mut Frame<B>,
    app: &mut App,
    text: Vec<Spans>,
    rules: Vec<Spans>,
    plugins: Vec<Spans>,
) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(app.title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.tabs.index);
    f.render_widget(tabs, chunks[0]);
    match app.tabs.index {
        0 => draw_first_tab(f, app, chunks[1], text),
        1 => draw_first_tab(f, app, chunks[1], rules),
        2 => draw_first_tab(f, app, chunks[1], plugins),
        _ => {}
    };
}

fn draw_first_tab<B>(f: &mut Frame<B>, _app: &mut App, area: Rect, text: Vec<Spans>)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Length(4)].as_ref())
        .split(area);
    draw_text(f, chunks[0], text);
}

fn draw_text<B>(f: &mut Frame<B>, area: Rect, text: Vec<Spans>)
where
    B: Backend,
{
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Logs",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
