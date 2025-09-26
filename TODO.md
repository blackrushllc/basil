Boom! 🌿 We now have:

* functions + calls + returns
* `IF/THEN[/ELSE]` + comparisons
* locals/params + recursion
* working VM frames & globals

🌱🌱🌱🌱🌱🌱🌱🌱🌱🌱

NEXT UP: control flow (loops) and boolean ops.

1. **WHILE/DO/END loops**

    * Add `Loop` (backward jump) opcode and compile:

      ```
      start:
        cond
        JumpIfFalse → end
        body
        Loop → start
      end:
      ```
    * ~30–40 lines across parser/compiler/vm.

2. **Boolean ops (`AND/OR/NOT`) with short-circuit**

    * Compile `A AND B` as `A; JumpIfFalse → end; B; end:`
    * Uses existing `JumpIfFalse`; minimal VM changes.

3. **Pretty errors with spans**

    * Map byte spans → line/column + caret diagnostics.
    * Massive QoL when the language grows.

4. **Disassembler/trace**

    * `basilc run --trace` to print executed opcodes & stack.
    * Super handy for debugging new control flow.

5. **String ops**

    * Add `&` (or `+`) for string concat and a `toString` for numbers.

6. **Standard lib seeds**

    * Built-ins like `clock()`, `len()`, `println()`, maybe `assert()` for tiny tests.

🌱🌱🌱🌱🌱🌱🌱🌱🌱🌱

