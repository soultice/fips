use crate::terminal_ui::debug::{LoggableNT, PrintInfo};
use colorgrad;
use gradient_tui_fork::{
    backend::Backend,
    gradient::BorderGradients,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, Paragraph, Tabs, Wrap},
    Frame,
};
use std::convert::TryFrom;
use std::sync::Arc;

use super::gradient_newtype::NewGradient;
use super::{state::State, App};

pub fn draw<B: Backend>(f: &mut Frame<'_, B>, app: &mut App<'_>, all_plugins: Vec<Spans<'_>>, rules_list: List<'_>) {
    let app_title = format!(
        "Fips──live on {} 😌, using config path: {}",
        app.opts.port,
        "not yet implemented" //
        //app.opts.config.clone().to_str().unwrap()
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
        top: Some(color_vec.clone()),
        bottom: Some(color_vec),
        ..Default::default()
    };

    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());

    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| {
            Spans::from(Span::styled(*t, Style::default().fg(Color::White)))
        })
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
            PrintInfo::Plain(info) => Spans::from(info.clone()),
            PrintInfo::Fips(info) => Spans::from(info),
        })
        .collect();

    f.render_widget(tabs, chunks[0]);

    match app.tabs.index {
        0 => draw_first_tab(f, app, chunks[1], main_info),
        1 => draw_info_tab(f, app, chunks[1], Arc::clone(&app.state)),
        2 => draw_rules_tab(f, app, chunks[1], rules_list),
        3 => draw_plugins_tab(f, app, chunks[1], all_plugins),
        _ => {}
    };
}

fn draw_plugins_tab<B>(
    f: &mut Frame<'_, B>,
    _app: &mut App<'_>,
    area: Rect,
    plugins: Vec<Spans<'_>>,
) where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Length(4)].as_ref())
        .split(area);
    draw_text(f, chunks[0], plugins, _app, "Loaded Plugins (by Rule)");
}

fn draw_first_tab<B>(
    f: &mut Frame<'_, B>,
    _app: &mut App<'_>,
    area: Rect,
    text: Vec<Spans<'_>>,
) where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Length(4)].as_ref())
        .split(area);
    draw_text(f, chunks[0], text, _app, "Logs");
}

fn draw_text<B>(f: &mut Frame<B>, area: Rect, text: Vec<Spans>, _app: &mut App, title: &str)
where
    B: Backend,
{
    let block = Block::default()
        .borders(
            Borders::LEFT | Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        )
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ))
        .border_gradients(_app.gradients.clone());

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_rules_tab<B>(
    f: &mut Frame<'_, B>,
    _app: &mut App<'_>,
    area: Rect,
    rules_list: List<'_>
) where
    B: Backend,
{
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Toggle Rules",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )).border_gradients(_app.gradients.clone());
    let constraints = vec![Constraint::Min(5)];
    let chunks = Layout::default()
        .constraints::<Vec<Constraint>>(constraints)
        .split(area);


    f.render_widget(rules_list.block(block), chunks[0])
}

fn draw_info_tab<B>(
    f: &mut Frame<'_, B>,
    _app: &mut App<'_>,
    area: Rect,
    state: Arc<State>,
) where
    B: Backend,
{
    let response_info: Vec<LoggableNT> =
        state.traffic_info.lock().unwrap().to_vec();

    let text: Vec<Text> = response_info.iter().map(Text::from).collect();

    let constraints: Vec<Constraint> = text
        .iter()
        .map(|t| Constraint::Max(u16::try_from(t.lines.len() + 2).unwrap()))
        .collect();

    let chunks = Layout::default()
        .constraints::<Vec<Constraint>>(constraints)
        .split(area);

    for (i, traffic_info) in response_info.iter().enumerate() {
        let title = traffic_info.to_string();
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            title,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(text[i].to_owned())
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[i]);
    }
}
