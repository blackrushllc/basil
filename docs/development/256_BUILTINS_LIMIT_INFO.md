### Short answer
No. The overflow happened because one mapping used the literal `256u8`, which is outside the `u8` range. We remapped Yore’s builtins into `150–155`, so the immediate problem is gone. The `u8` limit only applies to the number of VM builtins, not to the overall number of Basil keywords/features we can add.

### Why this doesn’t block adding more features
- Builtins are a fast path, not a requirement.
  - Only functions we intentionally mark as VM builtins consume an `Op::Builtin` ID (a single `u8`).
  - Most new functionality can be added as normal library functions or object methods without using a builtin ID at all.
- Feature gating reduces pressure.
  - Many IDs are conditionally compiled behind Cargo features. If a feature isn’t enabled, its IDs don’t matter.
- We still have room.
  - After moving Yore to `150–155`, there are still unused IDs. We also share IDs for aliases (e.g., `YORE_INIT%` and `YORE_INIT`).

### What happens if we ever get close to 256 builtins?
We have straightforward paths that avoid a hard cap:
- Don’t make it a builtin: compile to a regular function call instead of `Op::Builtin`.
- Add a new extended opcode when/if needed:
  - Example: `Op::BuiltinEx` that carries a `u16` ID (or a constant-table string key). This is a small, versioned bytecode change affecting the VM and compiler only.
- Introduce a host/global function registry (name → callback) so the compiler doesn’t need a numeric ID per function.

### Process/guardrails we’ll follow
- Keep a central table for builtin IDs and reserve ranges per subsystem to avoid collisions.
- CI lints to assert all builtin IDs are `<= 255` and to flag duplicates.
- Prefer library/objects over new builtins unless there’s a strong perf or integration reason.

### Bottom line
The overflow was a one-off mapping mistake, not a systemic limit on adding keywords. We can continue growing Basil safely, and if we ever truly need more builtin slots, we’ll switch to an extended opcode or a registry-based dispatch.