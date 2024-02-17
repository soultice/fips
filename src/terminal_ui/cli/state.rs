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

#[derive(Debug, Clone, PartialEq)]
pub enum MainError {
    Other { msg: String },
}

impl State {
    pub fn add_traffic_info(
        &self,
        traffic_info: LoggableNT,
    ) -> Result<(), MainError> {
        if let Ok(mut traffic) = self.traffic_info.lock() {
            traffic.insert(0, traffic_info);
            if traffic.len() > 20 {
                traffic.pop();
            }
        }
        Ok(())
    }

    pub fn add_message(&self, message: PrintInfo) -> Result<(), MainError> {
        if let Ok(mut messages) = self.messages.lock() {
            messages.insert(0, message);
            if messages.len() > 200 {
                messages.pop();
            }
        }
        Ok(())
    }
}
