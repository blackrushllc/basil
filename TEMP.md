# Feature Request: WHILE Loop with BREAK and CONTINUE, and Boolean Constants

Please add support in Basil for a `WHILE … BEGIN … END` looping structure. This loop should check the condition at the start of each iteration, and the block ends with our standard `END` keyword (no `WEND` or `LOOP`).

Inside a `WHILE` block, the language should support two new flow control statements:

* **`BREAK`** → immediately exit the current loop.
* **`CONTINUE`** → skip directly to the next iteration of the current loop.

Also, please add built-in constants **`TRUE`** and **`FALSE`**, which act as Boolean values. These should behave equivalently to `1` and `0`, so they can be used in numeric contexts as well.

This will allow patterns such as infinite loops (`WHILE TRUE BEGIN … END`) or sentinel-controlled loops (`WHILE condition BEGIN … END`) without needing extra keywords.

### Example usage cases (for testing)

1. **Conditional loop**

```
WHILE x < 10 BEGIN
    PRINT x
    x = x + 1
END
```

2. **Infinite loop with BREAK**

```
WHILE TRUE BEGIN
    INPUT line$
    IF line$ = "quit" THEN BEGIN
        BREAK
    END
    PRINT line$
END
```

Or equivalently:

```
WHILE 1 BEGIN
    INPUT line$
    IF line$ = "quit" THEN BREAK;
    END
    PRINT line$
END
```

3. **Using CONTINUE**

```
WHILE i < 5 BEGIN
    i = i + 1
    IF i = 3 THEN BEGIN
        CONTINUE
    END
    PRINT i
END
```

Or equivalently:

```
WHILE i < 5 BEGIN
    i = i + 1
    IF i = 3 THEN CONTINUE;
    PRINT i
END

4. **FALSE as never-enter condition**

```
WHILE FALSE BEGIN
    PRINT "You should never see this"
END
```

Please ensure these examples compile and run as expected once the feature is implemented.
