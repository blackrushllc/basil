use basil_atomic::*;
use serde_json::Value;

#[test]
fn test_manifest_deserialization() {
    let json = include_str!("fixtures/manifest-v1.json");
    let manifest: AtomicGameManifest =
        serde_json::from_str(json).expect("Failed to deserialize manifest");

    assert_eq!(manifest.format, ATOMIC_GAME_FORMAT);
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.runtime_version, 1);
    assert_eq!(manifest.game_id, "haunted-door");
    assert_eq!(manifest.title, "Haunted Door");
    assert_eq!(manifest.program_url, "/api/atomic-games/program-v1.json");
    assert_eq!(
        manifest.poster_url,
        Some("/api/atomic-games/assets/haunted-door.jpg".to_string())
    );
}

#[test]
fn test_manifest_serialization() {
    let manifest = AtomicGameManifest {
        format: ATOMIC_GAME_FORMAT.to_string(),
        format_version: 1,
        runtime_version: 1,
        game_id: "test-game".to_string(),
        revision: 2,
        title: "Test Game".to_string(),
        program_url: "/test.json".to_string(),
        poster_url: None,
    };

    let json = serde_json::to_string(&manifest).expect("Failed to serialize manifest");
    let value: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["format"], ATOMIC_GAME_FORMAT);
    assert_eq!(value["formatVersion"], 1);
    assert_eq!(value["gameId"], "test-game");
}

#[test]
fn test_program_deserialization() {
    let json = include_str!("fixtures/program-v1.json");
    let program: AtomicGameProgram =
        serde_json::from_str(json).expect("Failed to deserialize program");

    assert_eq!(program.format, ATOMIC_GAME_IR_FORMAT);
    assert_eq!(program.version, 1);
    assert_eq!(program.entry_point, "start");
    assert_eq!(program.instructions.len(), 22);

    // Check some instructions
    match &program.instructions[0] {
        AtomicInstruction::Label { name, line } => {
            assert_eq!(name, "start");
            assert_eq!(*line, 1);
        }
        _ => panic!("Expected LABEL"),
    }

    match &program.instructions[21] {
        AtomicInstruction::End { line } => {
            assert_eq!(*line, 26);
        }
        _ => panic!("Expected END"),
    }
}

#[test]
fn test_runtime_error_deserialization() {
    let json = include_str!("fixtures/runtime-error-v1.json");
    let error: AtomicRuntimeError =
        serde_json::from_str(json).expect("Failed to deserialize error");

    assert_eq!(error.error_type, ATOMIC_RUNTIME_ERROR_TYPE);
    assert_eq!(error.message, "Unknown label: basement");
    assert_eq!(error.instruction, 12);
    assert_eq!(error.source_line, 18);
}

#[test]
fn test_validation_valid_program() {
    let json = include_str!("fixtures/program-v1.json");
    let program: AtomicGameProgram = serde_json::from_str(json).unwrap();

    let result = validate_atomic_program(&program);
    assert!(result.is_ok(), "Validation failed: {:?}", result.err());
}

#[test]
fn test_validation_missing_entry_point() {
    let json = include_str!("fixtures/program-invalid-missing-label.json");
    let program: AtomicGameProgram = serde_json::from_str(json).unwrap();

    let result = validate_atomic_program(&program);
    assert!(result.is_err());
    let errors = result.err().unwrap();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("Entry point 'missing' does not exist")));
}

#[test]
fn test_validation_unknown_jump_target() {
    let mut program = AtomicGameProgram::default();
    program.instructions.push(AtomicInstruction::Label {
        name: "start".to_string(),
        line: 1,
    });
    program.instructions.push(AtomicInstruction::Goto {
        target: "basement".to_string(),
        line: 2,
    });

    let result = validate_atomic_program(&program);
    assert!(result.is_err());
    let errors = result.err().unwrap();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("references unknown label 'basement'")));
}

#[test]
fn test_validation_duplicate_labels() {
    let mut program = AtomicGameProgram::default();
    program.instructions.push(AtomicInstruction::Label {
        name: "start".to_string(),
        line: 1,
    });
    program.instructions.push(AtomicInstruction::Label {
        name: "start".to_string(),
        line: 2,
    });
    program
        .instructions
        .push(AtomicInstruction::End { line: 3 });

    let result = validate_atomic_program(&program);
    assert!(result.is_err());
    let errors = result.err().unwrap();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("Duplicate label 'start'")));
}

#[test]
fn test_validation_invalid_add_value() {
    let mut program = AtomicGameProgram::default();
    program.instructions.push(AtomicInstruction::Label {
        name: "start".to_string(),
        line: 1,
    });
    program.instructions.push(AtomicInstruction::Add {
        variable: "score".to_string(),
        value: serde_json::Value::String("banana".to_string()),
        line: 2,
    });
    program
        .instructions
        .push(AtomicInstruction::End { line: 3 });

    let result = validate_atomic_program(&program);
    assert!(result.is_err());
    let errors = result.err().unwrap();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("ADD requires a numeric value")));
}

#[test]
fn test_deserialization_unknown_op() {
    let json = include_str!("fixtures/program-invalid-unknown-op.json");
    let result: Result<AtomicGameProgram, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Expected deserialization error for unknown op"
    );
}
