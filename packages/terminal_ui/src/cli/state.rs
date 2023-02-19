use configuration::Configuration;
use plugin_registry::ExternalFunctions;

use std::{
    sync::{Arc, Mutex},
};

use crate::debug::{PrintInfo, TrafficInfo};

pub struct State {
    pub messages: Mutex<Vec<PrintInfo>>,
    pub plugins: Arc<Mutex<ExternalFunctions>>,
    pub configuration: Arc<Mutex<Configuration>>,
    pub traffic_info: Mutex<Vec<TrafficInfo>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MainError {
    Other { msg: String },
}

impl State {
    pub fn add_traffic_info(&self, traffic_info: TrafficInfo) -> Result<(), MainError> {
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
