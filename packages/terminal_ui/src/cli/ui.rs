use crate::cli::state::State;
use crate::cli::App;
use crate::debug::{PrintInfo, LoggableNT};
use colorgrad;
use std::convert::TryFrom;
use std::sync::Arc;
use gradient_tui_fork::{
    backend::Backend,
    gradient::BorderGradients,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, Paragraph, Tabs, Wrap},
    Frame,
};
use super::config_newtype::ConfigurationNewtype;
use super::gradient_newtype::NewGradient;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let app_title = format!(
        "Fips──live on {} 😌, using config path: {}",
        app.opts.port,
        app.opts.config.clone().to_str().unwrap()
    );

    let gradient = colorgrad::CustomGradient::new()
        .colors(&[
            colorgrad::Color::from_rgba8(255, 255, 255, 0),
            colorgrad::Color::from_rgba8(0, 0, app.glow_interval, 0),
            colorgrad::Color::from_rgba8(0, 0, app.glow_interval, 0),
            colorgrad::Color::from_rgba8(0, 0, app.glow_interval, 0),
            colorgrad::Color::from_rgba8(255, 255, 255, 0),
        ])
        .build()
        .ok();

    let colors = gradient
        .expect("where colors?")
        .colors(f.size().width as usize)
        .iter()
        .map(|c| c.clone().into())
        .collect::<Vec<NewGradient>>();

    let color_vec: Vec<Color> = colors.into_iter().map(|c| c.into()).collect();

    app.gradients = BorderGradients {
        bottom: Some(color_vec.clone()),
        top: Some(color_vec),
        ..Default::default()
    };

    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());

    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::White))))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::TOP).title(app_title))
        .highlight_style(Style::default().fg(Color::Blue))
        .select(app.tabs.index);

    let main_info = app
        .state
        .messages
        .lock()
        .unwrap()
        .iter()
        .map(|x| match x {
            PrintInfo::PLAIN(info) => Spans::from(info.clone()),
            PrintInfo::FIPS(info) => Spans::from(info),
        })
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
        1 => draw_info_tab(f, app, chunks[1], Arc::clone(&app.state)),
        2 => draw_rules_tab(f, app, chunks[1], Arc::clone(&app.state)),
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
    draw_text(f, chunks[0], text, _app);
}

fn draw_text<B>(f: &mut Frame<B>, area: Rect, text: Vec<Spans>, _app: &mut App)
where
    B: Backend,
{
    let block = Block::default()
        .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
        .title(Span::styled(
            "Logs",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ))
        .border_gradients(_app.gradients.clone());

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_rules_tab<B>(f: &mut Frame<B>, _app: &mut App, area: Rect, state: Arc<State>)
where
    B: Backend,
{
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Toggle Rules",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    ));
    let constraints = vec![Constraint::Min(5)];
    let chunks = Layout::default()
        .constraints(constraints.as_ref())
        .split(area);

    let wrapper = ConfigurationNewtype(&state.configuration);
    let list = List::from(wrapper).block(block);

    f.render_widget(list, chunks[0])
}

fn draw_info_tab<B>(f: &mut Frame<B>, _app: &mut App, area: Rect, state: Arc<State>)
where
    B: Backend,
{
    let response_info: Vec<LoggableNT> = state.traffic_info.lock().unwrap().to_vec();

    let text: Vec<Text> = response_info
        .iter()
        .map(Text::from)
        .collect();

    let constraints: Vec<Constraint> = text
        .iter()
        .map(|t| Constraint::Max(u16::try_from(t.lines.len() + 2).unwrap()))
        .collect();

    let chunks = Layout::default()
        .constraints(constraints.as_ref())
        .split(area);

    for (i, traffic_info) in response_info.iter().enumerate() {
        let title = traffic_info.to_string();
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
