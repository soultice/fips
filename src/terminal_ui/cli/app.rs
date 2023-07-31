use gradient_tui_fork::gradient::BorderGradients;
use std::sync::Arc;

use crate::{utility::options::CliOptions, terminal_ui::util::TabsState};

use super::state::State;

const REQUEST_TAB_NAME: &str = "Requests";
const TRAFFIC_TAB_NAME: &str = "Traffic";
const RULES_TAB_NAME: &str = "Loaded Rules";
const PLUGINS_TAB_NAME: &str = "Loaded Plugins";

pub struct App<'a> {
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub show_chart: bool,
    pub enhanced_graphics: bool,
    pub state: Arc<State>,
    pub opts: CliOptions,
    pub glow_interval: u8,
    pub gradients: BorderGradients,
    direction: Dir,
}

#[derive(PartialEq, Eq)]
enum Dir {
    Add,
    Substract,
}

impl<'a> App<'a> {
    pub fn new(enhanced_graphics: bool, state: Arc<State>, opts: CliOptions) -> App<'a> {
        App {
            should_quit: false,
            tabs: TabsState::new(vec![
                REQUEST_TAB_NAME,
                TRAFFIC_TAB_NAME,
                RULES_TAB_NAME,
                PLUGINS_TAB_NAME,
            ]),
            show_chart: true,
            enhanced_graphics,
            state,
            opts,
            glow_interval: 0,
            direction: Dir::Add,
            gradients: BorderGradients::default(),
        }
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.direction == Dir::Add {
            if let Some(s) = self.glow_interval.checked_add(5) {
                self.glow_interval = s;
            } else {
                self.direction = Dir::Substract;
                self.glow_interval = 250;
            }
        } else if let Some(s) = self.glow_interval.checked_sub(5) {
            self.glow_interval = s;
        } else {
            self.direction = Dir::Add;
            self.glow_interval = 0;
        }

        Ok(())
    }
}
