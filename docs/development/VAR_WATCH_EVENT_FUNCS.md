## Q.Map a function(FUNC/FUNCTION) to any kind of variable for an "On Change" event

I'm thinking about adding a new feature to our Basil BASIC programming language interpreter and compiler.

My idea is to be able to map a function(FUNC/FUNCTION) to any kind of variable.  The return type of the function must match the data type of the variable.

This will register an event in some kind of event table.  If the variable is modified, then the function is invoked.  The return value of the function is used to update the variable if it has changed.  Obviously we do not want this to cause the event to fire again, so in this case the variable will change without firing the event.

Similarly, a SUB can also be mapped to a variable and there would be no return value so no update would occur in this case.

The command which performs the mapping could return a handle that could be used to cancel the event handler, or maybe restart it if it is still defined but not active.

The event handling would have to be recursive or nested so that an event routine could change a different variable and cause that event to fire within the context of the first handler.

This might be a stupid idea.  I'm not sure if it would be practical or if having Getter/Setting functions would be more appropriate or frankly just the same thing without the mess.

Within the context of a GUI, having graphical controls mapped to a variable which then cascades to the variable 'on change' event could be useful.

Within the context of a CLI or CGI program this might not make any sense or result in spaghetti code.

It could also be used in simple educational programs to teach beginners about event handline.

Can you think of any Practical applications for a feature like this?


## A. Yes — there are practical uses for it.


What you are describing is basically a mix of:

* **variable watchers / observers**
* **reactive state**
* **computed properties**
* **lightweight data binding**

So the idea is not stupid at all. It is a real programming pattern. The question is not whether it is useful, but whether it is worth the complexity inside **Basil**.

## Where it would be genuinely useful

### 1. GUI data binding

This is probably the strongest case.

A textbox, slider, checkbox, label, grid cell, etc. could all be bound to variables, and when one changes, related variables or controls update automatically.

Example:

```basic
name$ = ""
greeting$ = ""

WATCH name$, @UpdateGreeting()
FUNCTION UpdateGreeting$()
    RETURN "Hello, " + name$ + "!"
END FUNCTION
```

If `name$` changes from a textbox, `greeting$` could update automatically, and the label showing `greeting$` changes too.

That is a very real and useful feature in event-driven GUI code.

---

### 2. Form validation and normalization

A variable changes, and the watcher sanitizes or adjusts it.

Example uses:

* trim whitespace
* force uppercase
* clamp a number to a range
* normalize a phone number
* reject or warn on invalid values

Example:

```basic
age% = 0
WATCH age%, @NormalizeAge()

FUNCTION NormalizeAge%()
    IF age% < 0 THEN RETURN 0
    IF age% > 120 THEN RETURN 120
    RETURN age%
END FUNCTION
```

That is more elegant than sprinkling validation everywhere.

---

### 3. Derived/calculated values

Very useful for simple apps, educational projects, or dashboards.

Example:

* `subtotal` changes ? update `tax`
* `tax` or `subtotal` changes ? update `total`
* `width` or `height` changes ? update `area`

This is essentially spreadsheet-like behavior.

Example:

```basic
subtotal = 0
tax = 0
total = 0

WATCH subtotal, @RecalcTax()
WATCH subtotal, @RecalcTotal()
WATCH tax, @RecalcTotal()
```

That is practical in GUI apps, calculators, educational demos, and even business forms.

---

### 4. Keeping model and UI in sync

If Basil forms or controls become a bigger part of the ecosystem, this could be very attractive.

Examples:

* variable changes ? update label text
* selected item changes ? enable/disable buttons
* status variable changes ? recolor a panel
* progress variable changes ? redraw progress bar

This is basically the heart of modern UI frameworks, just in a simpler BASIC-style form.

---

### 5. Logging, tracing, and debugging

A watcher mapped to a `SUB` could act as a watchpoint.

Example:

```basic
WATCH balance, @LogBalanceChange()
```

Whenever `balance` changes:

* write to log
* print debug info
* record old/new values
* mark state as dirty

That can be very practical even outside GUI code.

---

### 6. Dirty flags / autosave triggers

In editors, settings tools, or forms:

* any important variable changes
* mark document as modified
* enable Save button
* schedule autosave

That is a solid practical application.

---

### 7. Teaching event-driven programming

You mentioned this already, and I agree.

For beginners, this can be a very understandable bridge from:

* plain sequential BASIC
* to event-driven thinking
* to GUI programming
* to reactive systems

“Whenever this variable changes, run this code” is simple to explain.

---

## Where it is less useful

### CLI / CGI

You are right that it is less compelling there.

For normal command-line or request/response code, explicit logic is often clearer:

```basic
x = x + 1
CALL UpdateTotals()
```

rather than hidden variable-triggered behavior.

In CLI or CGI, implicit reactions can become surprising and hard to debug.

So I would not pitch this as a general-purpose core language style. I would pitch it as:

* a **GUI/reactive programming feature**
* a **debugging/teaching feature**
* an **optional convenience tool**

---

## Getter/setter functions are not exactly the same thing

They overlap, but they are not the same.

### Getters/setters

These are explicit:

```basic
CALL SetAge(150)
```

You know exactly where logic happens.

### Variable watchers

These are implicit:

```basic
age% = 150
```

and then something fires behind the scenes.

So setters are usually:

* easier to reason about
* easier to debug
* safer for large systems

Watchers are usually:

* more convenient
* better for UI/reactive programming
* better for automatic propagation

So the two features solve different problems.

---

## The biggest risk: spaghetti and surprise behavior

This is the real danger.

If a variable change silently triggers code, and that code changes another variable, and that triggers more code, then it can become difficult to follow.

Example bad chain:

* `a` changes
* watcher updates `b`
* `b` watcher updates `c`
* `c` watcher updates `a`

Now you have loops, hidden coupling, and weird bugs.

So the feature is useful **only if the semantics are very disciplined**.

---

## If you do it, I would strongly recommend guardrails

### 1. Make it opt-in and explicit

Do not make all variables magically reactive.

Use a clear API like:

```basic
h% = WATCH(var, @Handler())
UNWATCH h%
SUSPENDWATCH h%
RESUMEWATCH h%
```

or similar.

---

### 2. Pass old/new values into the handler

That makes the feature far more useful.

Example conceptually:

```basic
SUB OnNameChange(old$, new$)
```

or

```basic
FUNCTION NormalizeName$(old$, new$)
```

That is much better than forcing the function to inspect globals.

---

### 3. Distinguish between “before change” and “after change”

These are two very different use cases.

* **before change**: validate, veto, normalize
* **after change**: log, redraw, propagate

You may eventually want both.

Example names:

* `ONCHANGE BEFORE`
* `ONCHANGE AFTER`

or internal watcher types.

---

### 4. Prevent self-recursion by default

You already noticed this.

If the watcher changes the same variable it is watching, that should not refire automatically unless explicitly allowed.

That is the correct default.

---

### 5. Allow nested events, but track depth

Nested reactions are useful, but infinite cycles are dangerous.

A simple event depth counter or cycle protection would help a lot.

Even a runtime safeguard like:

* maximum nested event depth
* cycle detection for the same variable in the same chain
* debug warning when a loop is suspected

would be very valuable.

---

### 6. Keep handles simple

Returning a handle is a good idea.

Something like:

```basic
h% = WATCH(total, @OnTotalChange())
UNWATCH h%
```

is clean and BASIC-friendly.

---

### 7. Consider a separate concept for computed variables

This may actually be cleaner than generic watchers for many cases.

Example:

```basic
COMPUTED total = subtotal + tax
COMPUTED greeting$ = "Hello, " + name$
```

That gives you spreadsheet/reactive behavior without arbitrary event spaghetti.

In many real cases, **computed/bound variables** are safer than full “run any SUB/FUNCTION on change”.

---

## My honest recommendation

I think this feature **does have practical value**, especially for:

* Basil GUI/forms
* educational demos
* reactive/calculated values
* debugging/logging
* light state synchronization

But I would not make it a loose, general “any variable can do anything” system first.

I would implement it in stages:

### Best first step

Add a small, explicit watcher system:

* watch variable
* call SUB or FUNCTION
* optional returned replacement value
* no self-refire
* nested allowed
* handle-based unregister

### Even better alongside it

Add a simpler, higher-level feature for common cases:

* computed variables
* control bindings
* maybe form-field validation hooks

That would give you the benefits without pushing users into spaghetti too quickly.

## Bottom line

The practical applications are real, especially in **GUI binding, validation, derived values, autosave/dirty state, logging, and education**.

For normal CLI/CGI code, it is less compelling and can become messy.

So I would say:

**good idea as an optional reactive/event feature**

not

**good idea as a general everyday programming style**


