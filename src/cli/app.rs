use crate::util::TabsState;
use crate::{Opts, State};
use std::sync::Arc;

pub struct App<'a> {
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub show_chart: bool,
    pub enhanced_graphics: bool,
    pub state: Arc<State>,
    pub opts: Opts,
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
        }
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
