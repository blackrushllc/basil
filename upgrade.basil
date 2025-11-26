' upgrade.bas
' Interactive upgrader / installer for BASIC or BASIL components.

CONST OS_WINDOWS = "W"
CONST OS_LINUX   = "L"
CONST OS_MAC     = "M"

CONST ITEM_BASIL    = "B"
CONST ITEM_COMPILER = "C"
CONST ITEM_WEBSERVER= "W"
CONST ITEM_ALL      = "A"

SUB Main()
    DIM osChoice$, itemChoice$, targetDir$
    DIM ok%

    PRINTLN "=== Basil / BASIC Upgrade Utility ==="
    PRINTLN ""

    osChoice$ = PromptChoice$("Select OS (W)indows (L)inux or (M)ac <L> : ", OS_LINUX, "WLM")
    itemChoice$ = PromptChoice$("Select item(s) to download/upgrade (B)asil, (C)ompiler, (W)ebserver, (A)ll <A> : ", ITEM_ALL, "BCWA")

    targetDir$ = DefaultTargetDir$(osChoice$)
    targetDir$ = PromptWithDefault$("Enter path for binaries <" + targetDir$ + "> : ", targetDir$)

    targetDir$ = Trim$(targetDir$)
    IF targetDir$ = "" THEN BEGIN
        PRINTLN "No target directory specified. Aborting."
        RETURN
    END IF

    PRINTLN ""
    PRINTLN "Summary:"
    PRINTLN "  OS:        ", DescribeOS$(osChoice$)
    PRINTLN "  Items:     ", DescribeItems$(itemChoice$)
    PRINTLN "  Target dir:", targetDir$
    PRINTLN ""

    PRINTLN "Press ENTER to download files, or 'c' to abort: ";
    DIM k$
    k$=INPUT$()
    IF k$ <> "" AND (UCASE$(LEFT$(k$, 1)) = "C") THEN BEGIN
        PRINTLN "Aborted by user."
        RETURN
    END IF

    ok% = PerformUpgrade%(osChoice$, itemChoice$, targetDir$)
    IF ok% = 0 THEN BEGIN
        PRINTLN ""
        PRINTLN "Upgrade completed successfully."
    ELSE BEGIN
        PRINTLN ""
        PRINTLN "Upgrade finished with errors. One or more components may have failed."
    END IF

END SUB


FUNCTION PromptWithDefault$(prompt$, def$)
    DIM line$
    PRINTLN prompt$;
    line$ = INPUT$()
    line$ = Trim$(line$)
    IF line$ = "" THEN BEGIN
        RETURN def$
    ELSE BEGIN
        RETURN line$
    END IF
END FUNCTION


FUNCTION PromptChoice$(prompt$, def$, valid$)
    DIM line$, ch$
    WHILE TRUE BEGIN
        PRINTLN prompt$;
        line$ = INPUT$()
        line$ = Trim$(line$)
        IF line$ = "" THEN BEGIN
            ch$ = UCASE$(def$)
        ELSE BEGIN
            ch$ = UCASE$(LEFT$(line$, 1))
        END IF

        IF InSet%(ch$, valid$) THEN BEGIN
            RETURN ch$
        ELSE BEGIN
            PRINTLN "Invalid choice. Please enter one of: "; valid$
        END IF
    END WHILE
END FUNCTION


FUNCTION InSet%(value$, valid$)
    DIM i%
    FOR i% = 1 TO LEN(valid$) BEGIN
        IF MID$(valid$, i%, 1) = value$ THEN BEGIN
            RETURN -1
        END IF
        END
    NEXT i%
   RETURN 0
END FUNCTION


FUNCTION Trim$(s$)
    DIM i%, j%
    i% = 1
    WHILE i% <= LEN(s$) AND MID$(s$, i%, 1) <= " " BEGIN
        i% = i% + 1
    END WHILE

    j% = LEN(s$)
    WHILE j% >= i% AND MID$(s$, j%, 1) <= " " BEGIN
        j% = j% - 1
    END WHILE

    IF j% < i% THEN BEGIN
        RETURN ""
    ELSE BEGIN
        RETURN MID$(s$, i%, j% - i% + 1)
    END IF
END FUNCTION


FUNCTION DescribeOS$(os$)
    SELECT CASE os$
        CASE OS_WINDOWS
            RETURN "Windows"
        CASE OS_LINUX
            RETURN "Linux"
        CASE OS_MAC
            RETURN "macOS"
        CASE ELSE
            RETURN "Unknown"
    END SELECT
END FUNCTION


FUNCTION DescribeItems$(item$)
    SELECT CASE item$
        CASE ITEM_BASIL
            RETURN "Basil / Basic interpreter only"
        CASE ITEM_COMPILER
            RETURN "Compiler only"
        CASE ITEM_WEBSERVER
            RETURN "Web server only"
        CASE ITEM_ALL
            RETURN "All components"
        CASE ELSE
            RETURN "Unknown set"
    END SELECT
END FUNCTION


FUNCTION DefaultTargetDir$(os$)
    DIM p$
    ' Try to use the path of the current executable if available.
    p$ = EXEPATH$()

    IF Trim$(p$) <> "" THEN BEGIN
        RETURN p$
    END IF

    SELECT CASE os$
        CASE OS_WINDOWS
           RETURN "C:\Program Files\Basil"
        CASE OS_LINUX
            RETURN "/usr/local/bin"
        CASE OS_MAC
            RETURN "/usr/local/bin"
        CASE ELSE
            RETURN "."
    END SELECT
END FUNCTION


FUNCTION PerformUpgrade%(os$, item$, targetDir$)
    DIM failures%

    IF item$ = ITEM_BASIL OR item$ = ITEM_ALL THEN BEGIN
        IF UpgradeComponent%(os$, "basil", targetDir$) <> 0 THEN BEGIN
            failures% = failures% + 1
        END IF
    END IF

    IF item$ = ITEM_COMPILER OR item$ = ITEM_ALL THEN BEGIN
        IF UpgradeComponent%(os$, "compiler", targetDir$) <> 0 THEN BEGIN    
            failures% = failures% + 1
        END IF
    END IF

    IF item$ = ITEM_WEBSERVER OR item$ = ITEM_ALL THEN BEGIN
        IF UpgradeComponent%(os$, "webserver", targetDir$) <> 0 THEN BEGIN
            failures% = failures% + 1
        END IF
    END IF

    IF failures% = 0 THEN BEGIN
        RETURN 0
     ELSE BEGIN
        RETURN failures%
    END IF
END FUNCTION


FUNCTION UpgradeComponent%(os$, component$, targetDir$)
    DIM url$, filename$, destPath$, rc%

    filename$ = ComponentFilename$(os$, component$)
    IF filename$ = "" THEN BEGIN
        PRINTLN "Skipping ", component$, " (no filename defined for this OS)."
        RETURN 1
    END IF

    url$ = ComponentURL$(os$, component$, filename$)
    destPath$ = JoinPath$(targetDir$, filename$)

    PRINTLN ""
    PRINTLN "Downloading ", component$, " from:"
    PRINTLN "  ", url$
    PRINTLN "to:"
    PRINTLN "  ", destPath$

    rc% = NET_DOWNLOAD_FILE%(url$, destPath$)
    IF rc% <> 0 THEN BEGIN
        PRINTLN "  ERROR: Download failed with code ", rc%
        RETURN 1
        RETURN
    END IF

    ' Mark as executable on Unix-like systems.
    IF os$ = OS_LINUX OR os$ = OS_MAC THEN BEGIN
        MakeExecutable(destPath$)
    END IF

    PRINTLN "  OK"
    RETURN 0
END FUNCTION


FUNCTION ComponentFilename$(os$, component$)
    DIM base$

    SELECT CASE component$
        CASE "basil"
            base$ = "basilc"
        CASE "compiler"
            base$ = "bcc"
        CASE "webserver"
            base$ = "basil-serve"
        CASE ELSE
            base$ = ""
    END SELECT

    IF base$ = "" THEN BEGIN
        RETURN ""
    END IF

    SELECT CASE os$
        CASE OS_WINDOWS
            RETURN base$ + ".exe"
        CASE ELSE
            RETURN base$
    END SELECT
END FUNCTION


FUNCTION ComponentURL$(os$, component$, filename$)
    ' TODO: Replace this base URL with your actual download endpoint.
    DIM base$

    ' Example layout:
    '   https://blackrushbasic.com/downloads/<os>/<filename>
    SELECT CASE os$
        CASE OS_WINDOWS
            base$ = "http://blackrushbasic.com/downloads/windows/"
        CASE OS_LINUX
            base$ = "http://blackrushbasic.com/downloads/linux/"
        CASE OS_MAC
            base$ = "http://blackrushbasic.com/downloads/macos/"
        CASE ELSE
            base$ = "http://blackrushbasic.com/downloads/"
    END SELECT

    IF RIGHT$(base$, 1) <> "/" THEN BEGIN
        base$ = base$ + "/"
    END IF

   RETURN base$ + filename$
END FUNCTION


SUB MakeExecutable(path$)
    ' Optional: make the binary executable on Unix-like systems.
    ' This assumes a SHELL built-in or similar capability exists.
    DIM cmd$
    cmd$ = "chmod +x " + QuotePath$(path$)
    ' If your BASIC has SHELL, you can enable this:
    ' SHELL cmd$
END SUB

FUNCTION QuotePath$(s$)
    ' Very simple quoting; you may want to improve this for Windows.
   RETURN "'" + s$ + "'"
END FUNCTION

FUNCTION JoinPath$(dir$, file$)
    IF dir$ = "" THEN BEGIN
       RETURN file$
    ELSE BEGIN
        IF RIGHT$(dir$, 1) = "/" OR RIGHT$(dir$, 1) = "\\" THEN BEGIN
        RETURN dir$ + file$
        ELSE BEGIN
        ' Simple heuristic: use "/" here; OS can normalize if needed.
        RETURN dir$ + "/" + file$
        END IF
    END IF
END FUNCTION





' --- Program entry point ---
Main()
