pub mod error;
pub mod then;
pub mod when;
pub mod with;

use std::str::FromStr;
use error::ConfigurationError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use eyre::Result;
use super::intermediary::Intermediary;
use crate::plugin_registry::ExternalFunctions;
use self::then::Then;
use self::when::When;
use self::with::With;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Rule {
    pub name: String,
    pub path: String,
    pub when: Option<When>,
    pub with: Option<With>,
    pub then: Then,
}

impl Rule {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub async fn apply(&self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<()> {
        if self.matches(intermediary).await {
            self.then.apply(intermediary, functions).await?;
        }
        Ok(())
    }

    pub fn apply_when(&self, intermediary: &mut Intermediary) -> bool {
        match &self.when {
            Some(when) => when.apply(intermediary),
            None => true,
        }
    }

    pub async fn apply_then(&self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<()> {
        self.then.apply(intermediary, functions).await
    }

    pub async fn matches(&self, intermediary: &Intermediary) -> bool {
        self.should_apply(intermediary).is_ok()
    }

    pub fn should_apply(&self, intermediary: &Intermediary) -> Result<(), ConfigurationError> {
        if let Some(ref when) = self.when {
            when.verify(intermediary)?;
        }
        Ok(())
    }
}
