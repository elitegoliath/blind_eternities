use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[allow(dead_code)]
pub enum GameEvent {
    ZoneChange {
        object_id: String,
        from_zone: String,
        to_zone: String,
    },
    PhaseBegan {
        phase: String, // Stringified Phase/Step enum
    },
    DamageDealt {
        source_id: String,
        target_id: String,
        amount: u32,
    },
    SpellCast {
        spell_name: String,
        controller: String,
    },
    #[serde(untagged)]
    Custom(serde_json::Value),
}
