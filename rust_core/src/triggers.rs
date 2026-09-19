use crate::models::Effect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PendingTrigger {
    pub source_id: String, // ID of the permanent that triggered this
    pub controller: String,
    pub effect: Effect, // What happens when the trigger resolves
    #[serde(default)]
    pub required_targets: usize, // Does this trigger need a target to go on the stack?
}
