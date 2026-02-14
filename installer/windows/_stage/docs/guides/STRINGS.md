# Strings in Basil

This document describes string literals and the new String Interpolation feature.

## String Literals

- Delimited by double quotes: "Hello".
- Supports common escapes: \" \\ \n \t \r, and any other character following a backslash is taken literally (e.g., \# → '#').
- Strings can be concatenated with the + operator. If either side is a string, Basil concatenates the string forms of both values.

## String Interpolation

String interpolation lets you embed Basil expressions directly inside a string literal using the syntax:

- `#{ <expr> }`

Examples:

```
PRINT "Hello #{name$}!"
PRINT "Wins=#{r@.Wins%} of #{r@.Wins% + r@.Losses%} (#{r@.WinRate()*100}%)"
PRINT "Today is #{DATE$()}"
```

Where it works:
- Anywhere a regular string literal is allowed (assignments, arguments, concatenations, etc.).

Escapes inside strings still work (\" \\ \n \t ...), and you can escape the interpolation delimiters:
- Literal `#{` → write `\#{`
- Literal `}`  → write `\}`

Braces in the interpolation must be balanced; braces inside nested strings within the expression do not count.

### Semantics

- Each `#{ expr }` is evaluated at runtime at the point where it appears in the string and converted to a string using the same rules as PRINT. Numbers format as usual; strings are unchanged; objects use their standard display form.
- Interpolated strings are compiled into ordinary string concatenations. No new VM opcodes are used.

This means the following are equivalent:

- Interpolated: `"Hello #{name$}!"`
- Concatenation: `"" + (name$) + "!"`

### Errors

If the interpolated form is malformed, you may see these errors (with the actual line number shown):
- `Unterminated interpolation: missing '}' after '#{' at line <n>.`
- `Empty interpolation not allowed: expected expression after '#{' at line <n>.`

### Tips

- TYPE$("Hi #{x%}") returns "STRING".
- DESCRIBE works as with any other string value.

### Windows paths and backslashes

On Windows, remember that backslash is the string escape character. That means sequences like "\n", "\t", and "\"" are special — and any other character after a backslash is taken literally but the backslash itself is consumed. For example, "\s" becomes just "s".

This affects path concatenation. If you write:

```
LET dest$ = EXEPATH$()
LET path$ = dest$ + "\splash.png"
```

you will actually get a file named "splash.png" appended directly after the last path segment (e.g., "...\\releasesplash.png"), because the "\\" before "s" was treated as an escape and removed.

Preferred options:
- Use forward slashes (Windows accepts them): `dest$ + "/splash.png"`
- Or escape the backslash: `dest$ + "\\splash.png"`

Example:
```
LET dest$ = EXEPATH$()
LET file$ = dest$ + "/splash.png"  ' or: dest$ + "\\splash.png"
PRINTLN file$
```

