REM FOR loop over a dictionary via an explicit keys list
LET person@ = { "first": "Ada", "last": "Lovelace", "born": 1815 }
LET keys@ = [ "first", "last", "born" ]

FOR i% = 1 TO LEN(keys@) {
  LET k$ = keys@[i%]
  PRINT k$ + ": " + person@[k$]
}
NEXT
