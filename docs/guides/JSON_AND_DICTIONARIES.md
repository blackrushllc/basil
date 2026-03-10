# JSON and Dictionary Support in Basil

Basil provides native support for JSON data through the `JSON` object, the `CGI` object, and first-class Dictionary methods.

## The `NULL` Value

Basil now includes a `NULL` keyword and value type. This is used to represent the absence of a value, particularly when accessing missing keys in a dictionary.

```basil
LET X = NULL
IF X == NULL THEN
    PRINTLN "X is null"
END IF
```

## JSON Object

The `JSON` built-in object provides methods for parsing and stringifying JSON data.

### `JSON.PARSE(json_string$)`
Parses a JSON string and returns a **Dictionary** object (or a List, String, Number, etc., depending on the JSON content).

### `JSON.STRINGIFY(value)`
Converts a Basil value (Dictionary, List, String, Number, or Boolean) into its JSON string representation.

```basil
LET RAW$ = "{ ""name"": ""Basil"", ""version"": 1.1 }"
LET DATA = JSON.PARSE(RAW$)
PRINTLN "Name: "; DATA["name"]

LET BACK$ = JSON.STRINGIFY(DATA)
PRINTLN "JSON: "; BACK$
```

## Dictionary Support

When you parse JSON or use `CGI.JSON_DATA()`, Basil returns a Dictionary. Dictionaries support safe indexing and several useful methods.

### Safe Indexing
Accessing a key that doesn't exist in a dictionary returns `NULL` instead of causing a runtime error.

```basil
LET VAL = DICT["missing_key"]
IF VAL == NULL THEN
    PRINTLN "Key was not found"
END IF
```

### Dictionary Methods

| Method | Description |
| :--- | :--- |
| `HAS(key$)` | Returns `1` if the key exists, `0` otherwise. |
| `GET(key$, default)` | Returns the value for the key, or the provided `default` if the key is missing. |
| `KEYS()` | Returns a List of all keys in the dictionary. |

Example:
```basil
IF DICT.HAS("user") THEN
    PRINTLN "User: "; DICT.GET("user", "Anonymous")
END IF

PRINTLN "Available keys: "; DICT.KEYS()
```

## CGI Object

The `CGI` object provides access to web request parameters when running in a CGI environment.

### `CGI.JSON_DATA()`
Returns a Dictionary containing all GET and POST parameters. Keys are parameter names, and values are strings.

```basil
LET PARAMS = CGI.JSON_DATA()
PRINTLN "Action: "; PARAMS.GET("action", "default")
```

## Robust Error Handling with `TRY...CATCH`

Basil's `TRY...CATCH` now correctly intercepts all runtime errors, including division by zero, missing methods, and type mismatches.

```basil
TRY
    LET X = 1 / 0
CATCH ERR$
    PRINTLN "Caught error: "; ERR$
END TRY
```
