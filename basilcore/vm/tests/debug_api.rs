use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use basil_vm::{VM};
use basil_vm::debug::{Debugger, DebugEvent};
use basil_bytecode::{Chunk, Program as BCProgram, Value, Op};

#[test]
fn breakpoint_and_output_events() {
    // Build a minimal program: line 1; print "Hello"; halt
    let mut chunk = Chunk::default();
    let cidx = chunk.add_const(Value::Str("Hello".to_string()));
    chunk.push_op(Op::SetLine); chunk.push_u16(1);
    chunk.push_op(Op::Const); chunk.push_u16(cidx);
    chunk.push_op(Op::Print);
    chunk.push_op(Op::Halt);
    let prog = BCProgram { chunk, globals: vec![] };

    let dbg = Debugger::new();
    // Watch events
    let rx = dbg.subscribe();
    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let dbg_for_thread = dbg.clone();
    let handle = thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            match ev {
                DebugEvent::Started => { events_clone.lock().unwrap().push("Started".into()); }
                DebugEvent::StoppedBreakpoint { file: _, line: _ } => { events_clone.lock().unwrap().push("Stopped".into()); dbg_for_thread.resume(); }
                DebugEvent::Output(s) => { events_clone.lock().unwrap().push(format!("Output:{s}")); }
                DebugEvent::Continued => { /* ignore */ }
                DebugEvent::Exited => { events_clone.lock().unwrap().push("Exited".into()); break; }
            }
        }
    });

    let mut vm = VM::new(prog);
    vm.set_script_path("test.basil".to_string());
    // Set breakpoint before attaching
    dbg.set_breakpoint("test.basil".to_string(), 1);
    vm.set_debugger(dbg.clone());

    // Run the VM
    vm.run().expect("vm run");

    // Give event thread a moment, then join
    thread::sleep(Duration::from_millis(10));
    let _ = handle.join();

    let evs = events.lock().unwrap().clone();
    assert!(evs.contains(&"Started".to_string()));
    assert!(evs.iter().any(|e| e.starts_with("Output:Hello")));
    assert!(evs.contains(&"Exited".to_string()));
}
