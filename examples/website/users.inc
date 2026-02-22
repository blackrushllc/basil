REM Basil/Basic CGI Users Library Include File

FUNC db_open%()
    // Try DB in script directory first
    db% = SQLITE_OPEN%("website.db")
    IF db% == 0 THEN
        // Fallback to a writable temp dir
        tmp$ = ENV$("TMPDIR")
        IF LEN(tmp$) == 0 THEN tmp$ = ENV$("TEMP")
        IF LEN(tmp$) == 0 THEN tmp$ = ENV$("TMP")
        IF LEN(tmp$) == 0 THEN tmp$ = "/tmp"
        // Join path safely
        IF RIGHT$(tmp$, 1) == "/" OR RIGHT$(tmp$, 1) == "\\" THEN
            path$ = tmp$ + "website.db"
        ELSE
            path$ = tmp$ + "/" + "website.db"
        END IF
        db% = SQLITE_OPEN%(path$)
    END IF
    IF db% == 0 THEN RETURN 0
    _ = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT UNIQUE, password TEXT)")
    RETURN db%
END FUNC

FUNC user_exists%(db%, username$)
    DIM rows$(0,0)
    rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username = '" + username$ + "' LIMIT 1")
    RETURN ARRAY_ROWS%(rows$) > 0
END FUNC

FUNC check_login%(db%, username$, password$)
    DIM rows$(0,0)
    rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username='" + username$ + "' AND password='" + password$ + "' LIMIT 1")
    RETURN ARRAY_ROWS%(rows$) > 0
END FUNC


FUNC create_user%(db%, username$, password$)
    _ = SQLITE_EXEC%(db%, "INSERT INTO users(username, password) VALUES ('" + username$ + "','" + password$ + "')")
    RETURN SQLITE_LAST_INSERT_ID%(db%)
END FUNC
