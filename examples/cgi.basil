#CGI_NO_HEADER
<?basil
  // Manual header mode: send headers explicitly, then a blank line
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
?>
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Basil CGI Demo</title>
</head>
<body>
  <h1>Hello, World</h1>
  <p>This page is rendered by a Basil CGI template.</p>

  <h2>Request parameters</h2>
  <p>Any GET or POST parameters will be listed below.</p>
  <ul>
  <?basil
    FOR EACH p$ IN REQUEST$()
      PRINT "<li>" + HTML$(p$) + "</li>\n";
    NEXT
  ?>
  </ul>
</body>
</html>
