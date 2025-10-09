#CGI_NO_HEADER
<?basil
  LET SITE_TITLE$ = "Basil Website Skeleton"
  FUNC send_header_ok_html() BEGIN
    PRINT "Status: 200 OK\r\n";
    PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
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
  FUNC cookie$(name$) BEGIN
    LET raw$ = ENV$("HTTP_COOKIE")
    IF LEN(raw$) == 0 THEN RETURN ""
    LET s$ = raw$
    LET i% = 1
    WHILE i% <= LEN(s$) BEGIN
      LET semi% = INSTR(s$, ";", i%-1)
      IF semi% == 0 THEN LET semi% = LEN(s$)
      LET part$ = TRIM$(MID$(s$, i%, semi% - i% + 1))
      LET eq% = INSTR(part$, "=")
      IF eq% > 0 THEN BEGIN
        LET ck$ = LEFT$(part$, eq%)
        LET cv$ = MID$(part$, eq%+2)
        IF ck$ == name$ THEN RETURN cv$
      END
      LET i% = semi% + 2
    END
    RETURN ""
  END
  FUNC send_header_redirect(loc$) BEGIN
    PRINT "Status: 302 Found\r\n";
    PRINT "Location: " + loc$ + "\r\n\r\n";
    RETURN 0;
  END

  LET user$ = cookie$("user")
  IF LEN(user$) == 0 THEN BEGIN
    LET __d% = send_header_redirect("login.basil");
    EXIT 0;
  END

  LET __d% = send_header_ok_html()
  LET __d% = layout_start("Your dashboard")
?>
<?basil PRINT READFILE$("views/logged_in.html"); ?>
<?basil
  PRINT "<p class=\"greet\">Welcome, <strong>" + HTML$(user$) + "</strong>!</p>\n";
  PRINT "<form method=\"post\" action=\"logout.basil\"><button class=\"btn\">Log out</button></form>\n";
  LET __d% = layout_end()
?>