use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", content = "details")]
pub enum EngineError {
    OutOfPhase(String),
    InsufficientMana(String),
    IllegalTarget(String),
    StackEmpty(String),
    InvalidPriority(String),
    StateError(String),
    Custom(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::OutOfPhase(msg) => write!(f, "Out of Phase: {}", msg),
            EngineError::InsufficientMana(msg) => write!(f, "Insufficient Mana: {}", msg),
            EngineError::IllegalTarget(msg) => write!(f, "Illegal Target: {}", msg),
            EngineError::StackEmpty(msg) => write!(f, "Stack Empty: {}", msg),
            EngineError::InvalidPriority(msg) => write!(f, "Invalid Priority: {}", msg),
            EngineError::StateError(msg) => write!(f, "State Error: {}", msg),
            EngineError::Custom(msg) => write!(f, "Engine Error: {}", msg),
        }
    }
}

impl std::error::Error for EngineError {}
