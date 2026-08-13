use serde::{Deserialize, Serialize};

pub const ATOMIC_RUNTIME_ERROR_TYPE: &str = "runtime_error";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicRuntimeError {
    #[serde(rename = "type")]
    pub error_type: String,

    pub message: String,

    pub instruction: usize,

    #[serde(rename = "sourceLine")]
    pub source_line: u32,
}

impl AtomicRuntimeError {
    pub fn new(message: String, instruction_index: usize, source_line: u32) -> Self {
        Self {
            error_type: ATOMIC_RUNTIME_ERROR_TYPE.to_string(),
            message,
            instruction: instruction_index,
            source_line,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomicVmState {
    #[serde(rename = "READY")]
    Ready,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "WAITING_FOR_CHOICE")]
    WaitingForChoice,
    #[serde(rename = "WAITING_FOR_KEY")]
    WaitingForKey,
    #[serde(rename = "FINISHED")]
    Finished,
    #[serde(rename = "ERROR")]
    Error,
}

/*
Future states:
- WAITING_FOR_TIMER
- WAITING_FOR_API
- WAITING_FOR_MEDIA
*/
