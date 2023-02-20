use utility::options::Opts;
use crate::cli::state::State;
use crate::util::TabsState;
use tui::gradient::BorderGradients;
use std::sync::Arc;

pub struct App<'a> {
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub show_chart: bool,
    pub enhanced_graphics: bool,
    pub state: Arc<State>,
    pub opts: Opts,
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
    pub fn new(enhanced_graphics: bool, state: Arc<State>, opts: Opts) -> App<'a> {
        App {
            should_quit: false,
            tabs: TabsState::new(vec![
                "Requests",
                "Traffic",
                "Loaded Rules",
                "Loaded Plugins",
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
        } else {
            if let Some(s) = self.glow_interval.checked_sub(5) {
                self.glow_interval = s;
            } else {
                self.direction = Dir::Add;
                self.glow_interval = 0;
            }
        }

        Ok(())
    }
}
