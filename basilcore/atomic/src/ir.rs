use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ATOMIC_GAME_IR_FORMAT: &str = "atomicflix-game-ir";
pub const ATOMIC_GAME_IR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicGameProgram {
    pub format: String,
    pub version: u32,

    #[serde(rename = "entryPoint")]
    pub entry_point: String,

    pub instructions: Vec<AtomicInstruction>,
}

impl Default for AtomicGameProgram {
    fn default() -> Self {
        Self {
            format: ATOMIC_GAME_IR_FORMAT.to_string(),
            version: ATOMIC_GAME_IR_VERSION,
            entry_point: "start".to_string(),
            instructions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum AtomicInstruction {
    #[serde(rename = "LABEL")]
    Label { name: String, line: u32 },

    #[serde(rename = "LAYOUT")]
    Layout { value: String, line: u32 },

    #[serde(rename = "IMAGE")]
    Image { value: String, line: u32 },

    #[serde(rename = "TITLE")]
    Title { value: String, line: u32 },

    #[serde(rename = "TEXT")]
    Text { value: String, line: u32 },

    #[serde(rename = "CHOICE")]
    Choice {
        text: String,
        target: String,
        line: u32,
    },

    #[serde(rename = "WAITCHOICE")]
    WaitChoice { line: u32 },

    #[serde(rename = "GOTO")]
    Goto { target: String, line: u32 },

    #[serde(rename = "END")]
    End { line: u32 },

    #[serde(rename = "SET")]
    Set {
        variable: String,
        value: Value,
        line: u32,
    },

    #[serde(rename = "ADD")]
    Add {
        variable: String,
        value: Value,
        line: u32,
    },

    #[serde(rename = "IF_EQ")]
    IfEq {
        variable: String,
        value: Value,
        target: String,
        line: u32,
    },

    #[serde(rename = "WAITKEY")]
    WaitKey {
        #[serde(skip_serializing_if = "Option::is_none")]
        variable: Option<String>,
        line: u32,
    },
}

impl AtomicInstruction {
    pub fn op_name(&self) -> &'static str {
        match self {
            AtomicInstruction::Label { .. } => "LABEL",
            AtomicInstruction::Layout { .. } => "LAYOUT",
            AtomicInstruction::Image { .. } => "IMAGE",
            AtomicInstruction::Title { .. } => "TITLE",
            AtomicInstruction::Text { .. } => "TEXT",
            AtomicInstruction::Choice { .. } => "CHOICE",
            AtomicInstruction::WaitChoice { .. } => "WAITCHOICE",
            AtomicInstruction::Goto { .. } => "GOTO",
            AtomicInstruction::End { .. } => "END",
            AtomicInstruction::Set { .. } => "SET",
            AtomicInstruction::Add { .. } => "ADD",
            AtomicInstruction::IfEq { .. } => "IF_EQ",
            AtomicInstruction::WaitKey { .. } => "WAITKEY",
        }
    }

    pub fn source_line(&self) -> u32 {
        match self {
            AtomicInstruction::Label { line, .. } => *line,
            AtomicInstruction::Layout { line, .. } => *line,
            AtomicInstruction::Image { line, .. } => *line,
            AtomicInstruction::Title { line, .. } => *line,
            AtomicInstruction::Text { line, .. } => *line,
            AtomicInstruction::Choice { line, .. } => *line,
            AtomicInstruction::WaitChoice { line } => *line,
            AtomicInstruction::Goto { line, .. } => *line,
            AtomicInstruction::End { line } => *line,
            AtomicInstruction::Set { line, .. } => *line,
            AtomicInstruction::Add { line, .. } => *line,
            AtomicInstruction::IfEq { line, .. } => *line,
            AtomicInstruction::WaitKey { line, .. } => *line,
        }
    }

    pub fn jump_target(&self) -> Option<&str> {
        match self {
            AtomicInstruction::Choice { target, .. } => Some(target),
            AtomicInstruction::Goto { target, .. } => Some(target),
            AtomicInstruction::IfEq { target, .. } => Some(target),
            _ => None,
        }
    }
}
