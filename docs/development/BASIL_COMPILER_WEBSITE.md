# PART 1: BASIL COMPILER WEBSITE

# 1) views/upload.html

Create this file at `examples/basilbasic.com/views/upload.html`:

```html
<!-- views/upload.html -->
<section class="wrap">
  <div class="intro">
    <h2>Compile a Basil file</h2>
    <p>Upload a single <code>.basil</code> file. We’ll compile it to bytecode and return a ZIP with your source and the <code>.basilx</code>.</p>
  </div>

  <!-- optional error slot; compile.basil will print into this spot -->
  <!-- {{ERROR_BLOCK}} -->

  <div class="card" style="max-width:560px">
    <form action="compile.basil" method="post" enctype="multipart/form-data">
      <label for="source">Choose a .basil file</label>
      <input id="source" name="source" type="file" accept=".basil" required>
      <p class="notice" style="margin-top:.75rem">
        Max size: 256 KB by default. Only <code>.basil</code> files.
      </p>
      <div style="margin-top:1rem">
        <button class="btn" type="submit">Compile & Download ZIP</button>
        <a class="btn secondary" href="user_home.basil">Cancel</a>
      </div>
    </form>
  </div>
</section>
```

# 2) views/logged_in.html (add a button)

Add this somewhere sensible (under your greeting/dashboard):

```html
<a class="btn" href="compile.basil">Compile a Basil file</a>
```

# 3) compile.basil (GET shows form; POST compiles → zips → downloads)

Drop this in `examples/basilbasic.com/compile.basil`. It uses a few **helper shims** in #4 so it’s runnable now; swap them later for real Basil built-ins as you add them (e.g., `FILE_EXISTS`, `CGI_SAVE_FILE%`, etc.). All redirects/links are **relative** for subfolder hosting.

```basil
' compile.basil — upload → compile → zip → download (relative paths, same-folder CGI)
#CGI_NO_HEADER

' ---------- CONFIG ----------
LET MAX_SIZE%   = 262144   ' 256 KB (override with env BASIL_WEB_MAX_UPLOAD)
LET TMP_ROOT$   = "tmp"    ' per-request subdir created inside here
LET ZIP_BASENAME$ = "project"

IF LEN(ENV$("BASIL_WEB_MAX_UPLOAD")) > 0 THEN LET MAX_SIZE% = VAL(ENV$("BASIL_WEB_MAX_UPLOAD"))

' ---------- AUTH GUARD ----------
IF LEN(get_cookie$("user")) == 0 THEN BEGIN
  PRINT "Status: 302 Found\r\n"
  PRINT "Location: login.basil\r\n\r\n"
  EXIT 0
END

' ---------- DISPATCH ----------
IF UCASE$(CGI_REQUEST_METHOD$()) == "POST" THEN
  GOTO :handle_post
ELSE
  GOTO :show_form
END

:show_form
  send_header_ok_html()
  layout_start("Upload & Compile")

  ' render the upload form (no error)
  PRINT READFILE$("views/upload.html")

  layout_end()
  EXIT 0

:handle_post
  DIM err$
  LET field$ = "source"

  ' Validate presence
  IF NOT CGI_HAS_FILE%(field$) THEN LET err$ = "No file uploaded."

  IF LEN(err$) == 0 THEN BEGIN
    LET fname$ = CGI_FILE_NAME$(field$)
    LET size%  = CGI_FILE_SIZE%(field$)
    IF size% <= 0 THEN LET err$ = "Empty upload."
    IF size% > MAX_SIZE% THEN LET err$ = "File too large (limit " + STR$(MAX_SIZE%) + " bytes)."

    ' Only .basil extension
    IF LEN(err$) == 0 THEN BEGIN
      LET lower$ = LCASE$(fname$)
      IF RIGHT$(lower$, 6) <> ".basil" THEN LET err$ = "Only .basil files are allowed."
    END
  END

  IF LEN(err$) > 0 THEN BEGIN
    PRINT "Status: 400 Bad Request\r\n"
    PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n"
    layout_start("Upload error")

    ' inject an error block before the form
    PRINT "<div class=""error"">"; HTML$(err$); "</div>"
    PRINT READFILE$("views/upload.html")

    layout_end()
    EXIT 0
  END

  ' Prep temp workdir
  LET req_id$   = gen_req_id$()
  LET workdir$  = make_workdir$(TMP_ROOT$, req_id$)
  IF LEN(workdir$) == 0 THEN GOTO :fatal_500

  LET src_path$ = workdir$ + "main.basil"
  LET out_path$ = workdir$ + "main.basilx"
  LET readme$   = workdir$ + "README.txt"
  LET zip_path$ = workdir$ + "artifact.zip"

  ' Save upload
  IF CGI_SAVE_FILE%(field$, src_path$) == 0 OR NOT FILE_EXISTS(src_path$) THEN BEGIN
    LET err$ = "Failed to save uploaded file."
    GOTO :respond_500
  END

  ' Compile (prefer bcc, fallback to basilc)
  DIM rc%
  rc% = run_compile%(workdir$, src_path$, out_path$, readme$)
  IF rc% <> 0 OR NOT FILE_EXISTS(out_path$) THEN BEGIN
    PRINT "Status: 422 Unprocessable Entity\r\n"
    PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n"
    layout_start("Compile failed")
    PRINT "<div class=""error""><strong>Compilation failed.</strong> Please fix your Basil code and try again.</div>"
    PRINT "<pre class=""notice"">"; HTML$(tail_file$(readme$, 2000)); "</pre>"
    PRINT READFILE$("views/upload.html")
    layout_end()
    cleanup_dir(workdir$)
    EXIT 0
  END

  ' Zip it up
  IF create_zip%(zip_path$, src_path$, out_path$, readme$) == 0 THEN BEGIN
    LET err$ = "Failed to create ZIP."
    GOTO :respond_500
  END

  ' Stream the download
  LET dl_name$ = ZIP_BASENAME$ + "-" + STR$(EPOCH%()) + ".zip"
  PRINT "Status: 200 OK\r\n"
  PRINT "Content-Type: application/zip\r\n"
  PRINT "Content-Disposition: attachment; filename="""; dl_name$; """\r\n\r\n"
  SEND_FILE(zip_path$)

  cleanup_dir(workdir$)
  EXIT 0

:fatal_500
  LET err$ = "Server error (unable to prepare temp directory)."

:respond_500
  PRINT "Status: 500 Internal Server Error\r\n"
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n"
  layout_start("Server error")
  PRINT "<div class=""error"">"; HTML$(err$); "</div>"
  PRINT READFILE$("views/upload.html")
  layout_end()
  EXIT 0

' ---------- HELPERS (shim versions; replace with real Basil APIs later) ----------

FUNCTION gen_req_id$()
  RETURN STR$(EPOCH%()) + "-" + RANDOMSTR$(6)
END

FUNCTION make_workdir$(root$, id$)
  LET path$ = root$ + "/" + id$ + "/"
  IF NOT FILE_EXISTS(root$) THEN MKDIR(root$)
  IF MKDIRS(path$) == 0 THEN RETURN ""
  RETURN path$
END

FUNCTION run_compile%(workdir$, src$, out$, readme$)
  ' Try bcc (bytecode compiler)
  DIM cmd$
  cmd$ = "bcc bc " + QUOTE$(src$) + " -o " + QUOTE$(out$)
  DIM rc%: rc% = SYS_EXEC%(cmd$, workdir$, readme$)
  IF rc% == 0 AND FILE_EXISTS(out$) THEN RETURN 0

  ' Fallback to basilc with a hypothetical flag
  cmd$ = "basilc --bytecode " + QUOTE$(src$) + " -o " + QUOTE$(out$)
  rc% = SYS_EXEC%(cmd$, workdir$, readme$)
  RETURN rc%
END

FUNCTION create_zip%(zip$, file1$, file2$, file3$)
  ' Prefer a native Basil ZIP helper if available:
  IF ZIP_CREATE%(zip$, file1$, file2$, file3$) == 1 THEN RETURN 1

  ' Otherwise shell out to zip (quiet, junk paths)
  DIM cmd$
  cmd$ = "zip -q -j " + QUOTE$(zip$) + " " + QUOTE$(file1$) + " " + QUOTE$(file2$) + " " + QUOTE$(file3$)
  RETURN (SYS_EXEC%(cmd$, "", "") == 0)
END

FUNCTION tail_file$(path$, limit%)
  IF NOT FILE_EXISTS(path$) THEN RETURN ""
  DIM s$: s$ = READFILE$(path$)
  IF LEN(s$) <= limit% THEN RETURN s$
  RETURN RIGHT$(s$, limit%)
END

SUB cleanup_dir(path$)
  ' Best-effort; replace with a real recursive delete when available
  ' DIR_DELETE_RECURSIVE(path$)
END
```

# 4) web_utils.basil (minimal shim helpers)

If you want to run **now**, add this optional shim beside your other CGI files (same folder). Replace these with real Basil built-ins later.

```basil
' web_utils.basil — temporary shims; Junie can replace with real implementations

' Return 1 if a path exists, else 0
FUNCTION FILE_EXISTS(path$)
  ' naive: try opening for read
  DIM h%: h% = FOPEN%(path$, "r")
  IF h% >= 0 THEN BEGIN
    FCLOSE(h%)
    RETURN 1
  END
  RETURN 0
END

' Make nested directories; return 1 on success
FUNCTION MKDIRS(path$)
  ' For now, try a shell call; replace with native impl
  RETURN (SYS_EXEC%("mkdir -p " + QUOTE$(path$), "", "") == 0)
END

' Execute a command; tee stdout/stderr to file if provided; return exit code
FUNCTION SYS_EXEC%(cmd$, cwd$, tee_to$)
  DIM full$
  IF LEN(cwd$) > 0 THEN
    full$ = "sh -lc " + QUOTE$("cd " + QUOTE$(cwd$) + " && " + cmd$ + " 2>&1 | tee -a " + QUOTE$(tee_to$))
  ELSE
    full$ = "sh -lc " + QUOTE$(cmd$ + " 2>&1 | tee -a " + QUOTE$(tee_to$))
  END
  ' If tee_to$ is empty, drop the tee:
  IF LEN(tee_to$) == 0 THEN BEGIN
    IF LEN(cwd$) > 0 THEN
      full$ = "sh -lc " + QUOTE$("cd " + QUOTE$(cwd$) + " && " + cmd$ + " 2>&1")
    ELSE
      full$ = "sh -lc " + QUOTE$(cmd$ + " 2>&1")
    END
  END
  RETURN SYSTEM%(full$)
END

' Stream file bytes to stdout (for downloads)
SUB SEND_FILE(path$)
  ' Replace with a native SEND_FILE when available
  DIM h%: h% = FOPEN%(path$, "rb")
  IF h% < 0 THEN EXIT SUB
  DIM buf$, n%
  DO
    buf$ = FREAD$(h%, 65536, n%)
    IF n% <= 0 THEN EXIT DO
    PRINT BIN$(buf$)
  LOOP
  FCLOSE(h%)
END

' Placeholder ZIP_CREATE% (0 = failure, 1 = success)
FUNCTION ZIP_CREATE%(zip$, f1$, f2$, f3$)
  RETURN 0  ' force compile.basil to fall back to shell "zip"
END
```

---

## How to test (quick)

1. While logged out, visit `compile.basil` → should 302 to `login.basil`.
2. Log in → `compile.basil` GET shows the upload form.
3. Upload a non-`.basil` file → neat error displayed.
4. Upload a broken Basil file → 422 + stderr tail in a `<pre>`.
5. Upload a valid Basil file → browser downloads `project-<epoch>.zip` containing `main.basil`, `main.basilx`, `README.txt`.

If you’d rather keep the helpers elsewhere, we can tuck `web_utils.basil` into a small `lib/` folder and `INCLUDE` it. Want me to package these into your repo’s `examples/basilbasic.com` layout exactly as paths, so you can just drop them in?
