use crate::ir::{
    AtomicGameProgram, AtomicInstruction, ATOMIC_GAME_IR_FORMAT, ATOMIC_GAME_IR_VERSION,
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct AtomicValidationError {
    pub message: String,
    pub instruction: Option<usize>,
    pub source_line: Option<u32>,
}

impl std::fmt::Display for AtomicValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.instruction, self.source_line) {
            (Some(idx), Some(line)) => {
                write!(f, "[Instruction {}, line {}] {}", idx, line, self.message)
            }
            (Some(idx), None) => write!(f, "[Instruction {}] {}", idx, self.message),
            (None, Some(line)) => write!(f, "[Line {}] {}", line, self.message),
            (None, None) => write!(f, "{}", self.message),
        }
    }
}

pub fn validate_atomic_program(
    program: &AtomicGameProgram,
) -> Result<(), Vec<AtomicValidationError>> {
    let mut errors = Vec::new();

    // Program-level validation
    if program.format != ATOMIC_GAME_IR_FORMAT {
        errors.push(AtomicValidationError {
            message: format!(
                "Invalid format: expected '{}', got '{}'",
                ATOMIC_GAME_IR_FORMAT, program.format
            ),
            instruction: None,
            source_line: None,
        });
    }

    if program.version != ATOMIC_GAME_IR_VERSION {
        errors.push(AtomicValidationError {
            message: format!(
                "Unsupported version: expected {}, got {}",
                ATOMIC_GAME_IR_VERSION, program.version
            ),
            instruction: None,
            source_line: None,
        });
    }

    if program.entry_point.is_empty() {
        errors.push(AtomicValidationError {
            message: "Entry point is empty".to_string(),
            instruction: None,
            source_line: None,
        });
    }

    if program.instructions.is_empty() {
        errors.push(AtomicValidationError {
            message: "No instructions found in program".to_string(),
            instruction: None,
            source_line: None,
        });
    }

    let mut labels = HashSet::new();
    let mut duplicate_labels = HashSet::new();

    // First pass: collect labels and basic instruction validation
    for (idx, inst) in program.instructions.iter().enumerate() {
        let line = inst.source_line();
        if line == 0 {
            errors.push(AtomicValidationError {
                message: "Source line must be greater than zero".to_string(),
                instruction: Some(idx),
                source_line: Some(line),
            });
        }

        match inst {
            AtomicInstruction::Label { name, .. } => {
                if name.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "Label name is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                } else if !labels.insert(name.clone()) {
                    duplicate_labels.insert(name.clone());
                }
            }
            AtomicInstruction::Choice { text, target, .. } => {
                if text.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "CHOICE text is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
                if target.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "CHOICE target is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
            }
            AtomicInstruction::Set { variable, .. } => {
                if variable.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "SET variable is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
            }
            AtomicInstruction::Add {
                variable, value, ..
            } => {
                if variable.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "ADD variable is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
                if !value.is_number() {
                    errors.push(AtomicValidationError {
                        message: "ADD requires a numeric value".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
            }
            AtomicInstruction::IfEq {
                variable, target, ..
            } => {
                if variable.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "IF_EQ variable is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
                if target.is_empty() {
                    errors.push(AtomicValidationError {
                        message: "IF_EQ target is empty".to_string(),
                        instruction: Some(idx),
                        source_line: Some(line),
                    });
                }
            }
            AtomicInstruction::WaitKey { variable, .. } => {
                if let Some(v) = variable {
                    if v.is_empty() {
                        errors.push(AtomicValidationError {
                            message: "WAITKEY variable is non-empty when present".to_string(),
                            instruction: Some(idx),
                            source_line: Some(line),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for name in duplicate_labels {
        errors.push(AtomicValidationError {
            message: format!("Duplicate label '{}'", name),
            instruction: None,
            source_line: None,
        });
    }

    if !program.entry_point.is_empty() && !labels.contains(&program.entry_point) {
        errors.push(AtomicValidationError {
            message: format!("Entry point '{}' does not exist", program.entry_point),
            instruction: None,
            source_line: None,
        });
    }

    // Second pass: validate jump targets
    for (idx, inst) in program.instructions.iter().enumerate() {
        if let Some(target) = inst.jump_target() {
            if !labels.contains(target) {
                errors.push(AtomicValidationError {
                    message: format!("Instruction {} references unknown label '{}'", idx, target),
                    instruction: Some(idx),
                    source_line: Some(inst.source_line()),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
