#CGI_NO_HEADER
<?basil

    LET SITE_TITLE$ = "Basil Website Skeleton"

    FUNC send_header_ok_html() BEGIN
        PRINT "Status: 200 OK\r\n";
        PRINT "Content-Type: text/html; charset=utf-8\r\n";
        PRINT "Cache-Control: no-store\r\n\r\n";
        RETURN 0;
    END

    FUNC set_cookie(name$, value$) BEGIN
        PRINT "Set-Cookie: " + name$ + "=" + value$ + "; Path=/; HttpOnly\r\n";
        RETURN 0;
    END

    FUNC send_header_redirect(loc$) BEGIN
        PRINT "Status: 302 Found\r\n";
        PRINT "Location: " + loc$ + "\r\n\r\n";
        RETURN 0;
    END

    FUNC layout_start(title$) BEGIN
        PRINT "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>" + HTML$(title$) + "</title>";
        PRINT "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">";
        PRINT "<link rel=\"stylesheet\" href=\"css/site.css\"></head><body><header class=\"top\"><div class=\"wrap\"><h1>" + HTML$(SITE_TITLE$) + "</h1></div></header><main class=\"wrap\">\n";
        RETURN 0;
    END

    FUNC layout_end() BEGIN
        PRINT "</main><footer class=\"foot\"><div class=\"wrap\"><small><a href=\"index.basil\">Home</a></small></div></footer><script src=\"js/site.js\"></script></body></html>\n";
        RETURN 0;
    END

    FUNC param$(name$) BEGIN
        FOR EACH kv$ IN REQUEST$()
          BEGIN
              LET eq% = INSTR(kv$, "=");
              IF eq% > 0 THEN BEGIN
                LET k$ = LEFT$(kv$, eq%);
                LET v$ = MID$(kv$, eq%+2);
                IF k$ == name$ THEN RETURN v$;
              END
          END
        NEXT
        RETURN "";
    END

    FUNC db_open%() BEGIN
        // Try DB in script directory first
        LET db% = SQLITE_OPEN%("website.db");
        IF db% == 0 THEN BEGIN
            // Fallback to a writable temp dir
            LET tmp$ = ENV$("TMPDIR");
            IF LEN(tmp$) == 0 THEN LET tmp$ = ENV$("TEMP");
            IF LEN(tmp$) == 0 THEN LET tmp$ = ENV$("TMP");
            IF LEN(tmp$) == 0 THEN LET tmp$ = "/tmp";
            // Join path safely
            LET sep$ = "/";
            IF RIGHT$(tmp$, 1) == "/" OR RIGHT$(tmp$, 1) == "\\" THEN
            BEGIN
                LET path$ = tmp$ + "website.db";

            ELSE

                LET path$ = tmp$ + "/" + "website.db";
            END
            LET db% = SQLITE_OPEN%(path$);
        END
        IF db% == 0 THEN RETURN 0;
        LET _ = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT UNIQUE, password TEXT)");
        RETURN db%;
    END

    FUNC create_user%(db%, username$, password$) BEGIN
        LET _ = SQLITE_EXEC%(db%, "INSERT INTO users(username, password) VALUES ('" + username$ + "','" + password$ + "')")
        RETURN SQLITE_LAST_INSERT_ID%(db%)
    END

    FUNC user_exists%(db%, username$) BEGIN
        DIM rows$(0,0)
        LET rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username = '" + username$ + "' LIMIT 1")
        RETURN ARRAY_ROWS%(rows$) > 0
    END

    LET method$ = ENV$("REQUEST_METHOD");

    IF method$ == "POST" THEN BEGIN

        LET err$ = "";

        LET u$ = param$("username");
        LET p$ = param$("password");
        LET p2$ = param$("password2");

        LET db% = db_open%();
        IF db% == 0 THEN BEGIN
            PRINT "Status: 500 Internal Server Error\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDB open failed";
            EXIT 0;
        END

        ' Validation without ELSE: set err$ on first failure
        IF LEN(u$) == 0 OR LEN(p$) == 0 THEN BEGIN
            LET err$ = "Please fill in all fields";
        END

        IF LEN(err$) == 0 THEN BEGIN
            IF p$ <> p2$ THEN BEGIN
                LET err$ = "Passwords do not match";
            END
        END

        IF LEN(err$) == 0 THEN BEGIN
            IF user_exists%(db%, u$) THEN BEGIN
                LET err$ = "That username is already taken";
            END
        END

        IF LEN(err$) == 0 THEN BEGIN
            LET _id% = create_user%(db%, u$, p$);

            IF id% == 0 THEN
                LET err$ = "Could not create user (duplicate username or DB error)."

            ' ... after you’ve validated input and created the user:
            IF _id% > 0 THEN BEGIN
              ' set the session cookie
              LET __d% = set_cookie("user", u$)

              ' close DB if it’s open
              SQLITE_CLOSE(db%)

              ' IMPORTANT: send an explicit redirect status before Location
              PRINT "Status: 302 Found\r\n"
              PRINT "Location: user_home.basil\r\n\r\n"
              EXIT 0
            END
        END
        SQLITE_CLOSE(db%);
    END

    LET __d% = send_header_ok_html()
    LET __d% = layout_start("Register")
?>
<?basil PRINT READFILE$("views/register.html"); ?>
<?basil
    //LET err$ = "Hello";

    IF LEN(err$) > 0 THEN PRINT "<p class=\"error\">aa" + HTML$(err$) + "aa</p>\n";

    PRINT "<p class=\"error\">xx" + err$ + "xx</p>\n";

    LET __d% = layout_end()
?>