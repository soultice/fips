use std::sync::Arc;
use tokio::sync::Mutex;
use eyre::Result;
use log::LevelFilter;
use crate::{
    configuration::configuration::Config,
};

pub struct State {
    configuration: Arc<Mutex<Config>>,
    messages: Arc<Mutex<Vec<String>>>,
    traffic: Arc<Mutex<Vec<String>>>,
    should_quit: Arc<Mutex<bool>>,
    log_level: Arc<Mutex<LevelFilter>>,
}

impl State {
    pub fn new(configuration: Arc<Mutex<Config>>) -> Self {
        Self {
            configuration,
            messages: Arc::new(Mutex::new(Vec::new())),
            traffic: Arc::new(Mutex::new(Vec::new())),
            should_quit: Arc::new(Mutex::new(false)),
            log_level: Arc::new(Mutex::new(LevelFilter::Info)),
        }
    }

    pub async fn get_configuration(&self) -> Result<Config> {
        Ok(self.configuration.lock().await.clone())
    }

    pub async fn set_configuration(&self, config: Config) -> Result<()> {
        *self.configuration.lock().await = config;
        Ok(())
    }

    pub async fn add_message(&self, message: String) -> Result<()> {
        self.messages.lock().await.push(message);
        Ok(())
    }

    pub async fn get_messages(&self) -> Result<Vec<String>> {
        Ok(self.messages.lock().await.clone())
    }

    pub async fn clear_messages(&self) -> Result<()> {
        self.messages.lock().await.clear();
        Ok(())
    }

    pub async fn add_traffic(&self, traffic: String) -> Result<()> {
        self.traffic.lock().await.push(traffic);
        Ok(())
    }

    pub async fn get_traffic(&self) -> Result<Vec<String>> {
        Ok(self.traffic.lock().await.clone())
    }

    pub async fn clear_traffic(&self) -> Result<()> {
        self.traffic.lock().await.clear();
        Ok(())
    }

    pub async fn set_should_quit(&self, should_quit: bool) -> Result<()> {
        *self.should_quit.lock().await = should_quit;
        Ok(())
    }

    pub async fn get_should_quit(&self) -> Result<bool> {
        Ok(*self.should_quit.lock().await)
    }

    pub async fn set_log_level(&self, level: LevelFilter) -> Result<()> {
        *self.log_level.lock().await = level;
        Ok(())
    }

    pub async fn get_log_level(&self) -> Result<LevelFilter> {
        Ok(*self.log_level.lock().await)
    }
} 