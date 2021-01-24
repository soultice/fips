use crate::cli::App;
use crate::{PrintInfo, State, TrafficInfo};
use futures::StreamExt;
use std::convert::TryFrom;
use std::sync::Arc;
use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let app_title = format!(
        "Moxy──live on {} 😌, using config path: {}",
        app.opts.port,
        app.opts.config.clone().to_str().unwrap()
    );
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
        .block(Block::default().borders(Borders::ALL).title(app_title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.tabs.index);

    let main_info = app
        .state
        .messages
        .lock()
        .unwrap()
        .iter()
        .map(|x| match x {
            PrintInfo::PLAIN(info) => Spans::from(info.clone()),
            PrintInfo::MOXY(info) => Spans::from(info),
        })
        .collect();

    let loaded_rules_info: Vec<Spans> = app
        .state
        .configuration
        .lock()
        .unwrap()
        .paths()
        .iter()
        .map(|p| Spans::from(Span::from(p.clone())))
        .collect();

    let loaded_plugins_info: Vec<Spans> = app
        .state
        .plugins
        .lock()
        .unwrap()
        .keys()
        .map(|e| Spans::from(Span::from(e.clone())))
        .collect();

    f.render_widget(tabs, chunks[0]);

    match app.tabs.index {
        0 => draw_first_tab(f, app, chunks[1], main_info),
        1 => draw_info_tab(f, app, chunks[1], &app.state.clone()),
        2 => draw_first_tab(f, app, chunks[1], loaded_rules_info),
        3 => draw_first_tab(f, app, chunks[1], loaded_plugins_info),
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

fn draw_info_tab<B>(f: &mut Frame<B>, _app: &mut App, area: Rect, state: &Arc<State>)
where
    B: Backend,
{
    let response_info: Vec<TrafficInfo> = state.traffic_info.lock().unwrap().to_vec();

    let text: Vec<Text> = response_info
        .iter()
        .map(|traffic_info| {
            let text = match traffic_info {
                TrafficInfo::OUTGOING_RESPONSE(i) => Text::from(i),
                TrafficInfo::INCOMING_RESPONSE(i) => Text::from(i),
                TrafficInfo::OUTGOING_REQUEST(i) => Text::from(i),
                TrafficInfo::INCOMING_REQUEST(i) => Text::from(i),
            };
            text
        })
        .collect();

    let mut constraints: Vec<Constraint> = text
        .iter()
        .map(|t| Constraint::Min(u16::try_from(t.lines.len() + 2).unwrap()))
        .collect();

    constraints.push(Constraint::Min(1));

    let chunks = Layout::default()
        .constraints(constraints.as_ref())
        .split(area);

    for (i, traffic_info) in response_info.iter().enumerate() {
        let title = match traffic_info {
            TrafficInfo::OUTGOING_RESPONSE(i) | TrafficInfo::INCOMING_RESPONSE(i) => {
                &i.response_type
            }
            TrafficInfo::OUTGOING_REQUEST(i) | TrafficInfo::INCOMING_REQUEST(i) => &i.request_type,
        };
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            title,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(text[i].clone())
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[i]);
    }
}
