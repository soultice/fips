use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    configuration::configuration::Config,
    terminal_ui::debug::{LoggableNT, PrintInfo},
};

pub struct State {
    pub messages: Mutex<Vec<PrintInfo>>,
    pub configuration: Arc<AsyncMutex<Config>>,
    pub traffic_info: Mutex<Vec<LoggableNT>>,
}

impl State {
    pub fn add_traffic_info(
        &self,
        traffic_info: LoggableNT,
    ) {
        if let Ok(mut traffic) = self.traffic_info.lock() {
            traffic.insert(0, traffic_info);
            if traffic.len() > 20 {
                traffic.pop();
            }
        }
    }

    pub fn add_message(&self, message: PrintInfo) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.insert(0, message);
            if messages.len() > 200 {
                messages.pop();
            }
        }
    }
}
