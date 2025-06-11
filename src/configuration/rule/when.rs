use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::configuration::types::Match;


#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct When {
    pub matches: Vec<Match>,
    pub probability: Option<f64>,
}

use super::error::ConfigurationError;
use crate::configuration::intermediary::Intermediary;
use rand::Rng;

impl When {
    pub fn apply(&self, intermediary: &Intermediary) -> bool {
        // Check probability first
        if let Some(prob) = self.probability {
            if prob < 1.0 {
                let mut rng = rand::thread_rng();
                if rng.gen::<f64>() > prob {
                    return false;
                }
            }
        }

        // All matches must pass
        self.matches.iter().all(|m| m.is_match(intermediary))
    }

    pub fn verify(&self, intermediary: &Intermediary) -> Result<(), ConfigurationError> {
        // Check probability first
        if let Some(prob) = self.probability {
            if prob < 0.0 || prob > 1.0 {
                return Err(ConfigurationError::InvalidProbability(prob));
            }
        }

        // Verify all matches
        for m in &self.matches {
            if let Err(e) = m.verify(intermediary) {
                return Err(e);
            }
        }
        Ok(())
    }
}

