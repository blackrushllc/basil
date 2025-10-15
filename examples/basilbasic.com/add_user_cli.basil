PRINTLN "Add user to website.db";

LET DB_PATH$ = "website.db";

FUNC db_open%() BEGIN
  // Try DB in script directory first
  LET db% = SQLITE_OPEN%(DB_PATH$);
  IF db% == 0 THEN BEGIN
    // Fallback to a writable temp dir
    LET tmp$ = ENV$("TMPDIR");
    IF LEN(tmp$) == 0 THEN LET tmp$ = ENV$("TEMP");
    IF LEN(tmp$) == 0 THEN LET tmp$ = ENV$("TMP");
    IF LEN(tmp$) == 0 THEN LET tmp$ = "/tmp";
    // Join path safely
    IF RIGHT$(tmp$, 1) == "/" OR RIGHT$(tmp$, 1) == "\\" THEN BEGIN
      LET DB_PATH$ = tmp$ + "website.db";
    ELSE
      LET DB_PATH$ = tmp$ + "/" + "website.db";
    END
    LET db% = SQLITE_OPEN%(DB_PATH$);
  END
  IF db% == 0 THEN RETURN 0;
  LET _ = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT UNIQUE, password TEXT)");
  RETURN db%;
END

// Escape single quotes for naive SQL concatenation
FUNC esc_sql$(s$) BEGIN
REM  LET out$ = "";
REM  FOR i% = 1 TO LEN(s$)
REM    LET ch$ = MID$(s$, i%, 1);
REM    IF ch$ == "'" THEN BEGIN
REM      LET out$ = out$ + "''";
REM    ELSE
REM      LET out$ = out$ + ch$;
REM    END
REM  NEXT i%
REM  RETURN out$;
RETURN s$;
END

// Open DB
LET db% = db_open%();
IF db% == 0 THEN BEGIN
  PRINTLN "Failed to open database (check permissions).";
  EXIT 1;
END
PRINTLN "Using DB file: ", DB_PATH$;

// Prompt for credentials
LET u$ = INPUT$("Enter username: ");
LET p$ = INPUT$("Enter password: ");

IF LEN(u$) == 0 OR LEN(p$) == 0 THEN BEGIN
  PRINTLN "Username and password are required.";
  SQLITE_CLOSE(db%);
  EXIT 1;
END

LET u2$ = esc_sql$(u$);
LET p2$ = esc_sql$(p$);

// Attempt insert
LET aff% = SQLITE_EXEC%(db%, "INSERT INTO users(username, password) VALUES ('" + u2$ + "','" + p2$ + "')");
LET id% = SQLITE_LAST_INSERT_ID%(db%);

IF id% > 0 THEN BEGIN
  PRINTLN "Inserted user id = ", id%, " (", u$, ")";
ELSE
  PRINTLN "Insert may have failed (id=", id%, ", affected=", aff%, "). If username must be unique, pick another.";
END

SQLITE_CLOSE(db%);
PRINTLN "Done. You can now test the website to read the users table.";
