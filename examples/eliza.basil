REM ELIZA in Basil (no line numbers). Uses INSTR-based rules and reflection.
REM Port guided by ELIZA.md. Prioritized keyword patterns, pronoun reflection, defaults.
REM Based on the original ELIZA in BASIC by Joseph Weizenbaum, 1966.
REM See also https://en.wikipedia.org/wiki/ELIZA for some history.

REM --- Globals ---
LET RP% = 12;            REM reflection pair count
DIM RF_IN$(RP%-1);       REM 0..RP%-1 (arrays are 0-based; LEN reports total elements)
DIM RF_OUT$(RP%-1);

REM Fill reflection pairs (normalized uppercase). Include IM->YOU ARE per ELIZA.md
LET RF_IN$(0) = "I";        LET RF_OUT$(0) = "YOU";
LET RF_IN$(1) = "ME";       LET RF_OUT$(1) = "YOU";
LET RF_IN$(2) = "MY";       LET RF_OUT$(2) = "YOUR";
LET RF_IN$(3) = "MINE";     LET RF_OUT$(3) = "YOURS";
LET RF_IN$(4) = "AM";       LET RF_OUT$(4) = "ARE";
LET RF_IN$(5) = "IM";       LET RF_OUT$(5) = "YOU ARE";
LET RF_IN$(6) = "YOU";      LET RF_OUT$(6) = "I";
LET RF_IN$(7) = "YOUR";     LET RF_OUT$(7) = "MY";
LET RF_IN$(8) = "YOURS";    LET RF_OUT$(8) = "MINE";
LET RF_IN$(9) = "ARE";      LET RF_OUT$(9) = "AM";
LET RF_IN$(10) = "MYSELF";  LET RF_OUT$(10) = "YOURSELF";
LET RF_IN$(11) = "YOURSELF";LET RF_OUT$(11) = "MYSELF";

DIM DEF$(11);            REM 12 default replies
LET DEF$(0) = "PLEASE TELL ME MORE.";
LET DEF$(1) = "LET'S TALK MORE ABOUT THAT.";
LET DEF$(2) = "HOW DOES THAT MAKE YOU FEEL?";
LET DEF$(3) = "WHY DO YOU SAY THAT?";
LET DEF$(4) = "I SEE. GO ON.";
LET DEF$(5) = "DOES THAT MEAN ANYTHING SPECIAL TO YOU?";
LET DEF$(6) = "CAN YOU ELABORATE?";
LET DEF$(7) = "DOES THIS HAPPEN OFTEN?";
LET DEF$(8) = "WHAT DO YOU THINK?";
LET DEF$(9) = "WHY DO YOU THINK THAT?";
LET DEF$(10) = "INTERESTING. PLEASE CONTINUE.";
LET DEF$(11) = "DO YOU FEEL THAT FREQUENTLY?";

LET next_i% = 0;        REM rotating index used for pseudo-random choices (1..N)

FUNC NEXT%(n%)
  LET next_i% = next_i% + 1;
  IF next_i% > n% THEN LET next_i% = 1;
  RETURN next_i%;
END FUNC

FUNC DEFAULT_REPLY()
  LET n% = LEN(DEF$);
  LET i% = NEXT%(n%);
  LET idx% = i% - 1;
  PRINT "ELIZA: ", DEF$(idx%);
  RETURN 0;
END FUNC

FUNC NORMALIZE$(s$)
  REM Uppercase, non-alnum -> space, collapse spaces, trim
  LET t$ = "";
  FOR i% = 1 TO LEN(s$) BEGIN
    LET c$ = MID$(s$, i%, 1);
    LET a% = ASC%(c$);
    IF a% >= 97 AND a% <= 122 THEN LET c$ = CHR$(a% - 32);
    LET a% = ASC%(c$);
    IF (a% >= 65 AND a% <= 90) OR (a% >= 48 AND a% <= 57) OR a% == 32 THEN BEGIN
      LET t$ = t$ + c$;
    ELSE
      LET t$ = t$ + " ";
    END
  END
  NEXT i%
  REM collapse spaces
  LET u$ = "";
  LET prv% = 1;
  FOR j% = 1 TO LEN(t$) BEGIN
    LET c$ = MID$(t$, j%, 1);
    IF c$ == " " THEN BEGIN
      IF prv% == 0 THEN BEGIN LET u$ = u$ + " "; END
      LET prv% = 1;
    ELSE BEGIN
      LET u$ = u$ + c$;
      LET prv% = 0;
    END
  END
  NEXT j%
  REM trim
  WHILE LEN(u$) > 0 AND LEFT$(u$, 1) == " " BEGIN LET u$ = MID$(u$, 2); END
  WHILE LEN(u$) > 0 AND RIGHT$(u$, 1) == " " BEGIN LET u$ = LEFT$(u$, LEN(u$)-1); END
  RETURN u$;
END FUNC

FUNC REFLECT$(x$)
  REM Trim leading/trailing spaces first
  LET xs$ = x$;
  WHILE LEN(xs$) > 0 AND LEFT$(xs$, 1) == " " BEGIN LET xs$ = MID$(xs$, 2); END
  WHILE LEN(xs$) > 0 AND RIGHT$(xs$, 1) == " " BEGIN LET xs$ = LEFT$(xs$, LEN(xs$)-1); END
  IF LEN(xs$) == 0 THEN RETURN "THAT";

  LET rx$ = "";
  LET w$ = "";
  LET xs2$ = xs$ + " ";
  FOR i% = 1 TO LEN(xs2$) BEGIN
    LET c$ = MID$(xs2$, i%, 1);
    IF c$ != " " THEN BEGIN
      LET w$ = w$ + c$;
    ELSE BEGIN
      IF LEN(w$) > 0 THEN BEGIN
        LET found% = 0;
        FOR j% = 0 TO RP%-1 BEGIN
          IF w$ == RF_IN$(j%) THEN BEGIN
            LET rx$ = rx$ + RF_OUT$(j%) + " ";
            LET found% = 1;
            LET j% = RP%-1;  REM break
          END
        END
        NEXT j%
        IF found% == 0 THEN LET rx$ = rx$ + w$ + " ";
      END
      LET w$ = "";
    END
  END
  NEXT i%
  IF LEN(rx$) > 0 AND RIGHT$(rx$, 1) == " " THEN LET rx$ = LEFT$(rx$, LEN(rx$)-1);
  RETURN rx$;
END FUNC

PRINT "ELIZA: HELLO. HOW ARE YOU FEELING TODAY?";

WHILE TRUE BEGIN
  PRINT "\nYOU: ";
  LET raw$ = INPUT$();
  IF LEN(raw$) == 0 THEN CONTINUE;
  LET norm$ = NORMALIZE$(raw$);
  IF norm$ == "BYE" OR norm$ == "GOODBYE" OR norm$ == "QUIT" THEN BEGIN
    PRINT "ELIZA: GOODBYE. IT WAS NICE TALKING TO YOU.";
    BREAK;
  END
  IF LEN(norm$) == 0 THEN BEGIN
    DEFAULT_REPLY();
    CONTINUE;
  END

  REM Surround with padding to simplify word-boundary matches; two leading spaces avoid INSTR ambiguity at index 0
  LET s$ = "  " + norm$ + " ";

  LET responded% = 0;

  REM 1) * I FEEL *
  LET k$ = " I FEEL ";
  LET p% = INSTR(s$, k$);
  IF p% > 0 AND responded% == 0 THEN BEGIN
    LET x$ = MID$(s$, p% + LEN(k$) + 1);
    LET rx$ = REFLECT$(x$);
    LET i% = NEXT%(3);
    IF i% == 1 THEN PRINT "ELIZA: WHY DO YOU FEEL ", rx$, "?";
    IF i% == 2 THEN PRINT "ELIZA: DO YOU OFTEN FEEL ", rx$, "?";
    IF i% == 3 THEN PRINT "ELIZA: WHAT MAKES YOU FEEL ", rx$, "?";
    LET responded% = 1;
  END

  REM 2) * I AM *
  LET k$ = " I AM ";
  LET p% = INSTR(s$, k$);
  IF p% > 0 AND responded% == 0 THEN BEGIN
    LET x$ = MID$(s$, p% + LEN(k$) + 1);
    LET rx$ = REFLECT$(x$);
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: HOW LONG HAVE YOU BEEN ", rx$, "?";
    IF i% == 2 THEN PRINT "ELIZA: WHY DO YOU THINK YOU ARE ", rx$, "?";
    LET responded% = 1;
  END

  REM 3) * I * YOU *
  LET k1% = INSTR(s$, " I ");
  LET k2% = INSTR(s$, " YOU ");
  IF k1% > 0 AND k2% > k1% AND responded% == 0 THEN BEGIN
    LET midlen% = k2% - (k1% + 3);
    IF midlen% < 0 THEN LET midlen% = 0;
    LET mid$ = MID$(s$, k1% + 3 + 1, midlen%);
    LET rmid$ = REFLECT$(mid$);
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: DO YOU REALLY THINK YOU ", rmid$, " ME?";
    IF i% == 2 THEN PRINT "ELIZA: PERHAPS IN YOUR FANTASIES YOU ", rmid$, " ME.";
    LET responded% = 1;
  END

  REM 4) * YOU * ME *
  LET k1% = INSTR(s$, " YOU ");
  LET k2% = INSTR(s$, " ME ");
  IF k1% > 0 AND k2% > k1% AND responded% == 0 THEN BEGIN
    LET midlen% = k2% - (k1% + 5);
    IF midlen% < 0 THEN LET midlen% = 0;
    LET mid$ = MID$(s$, k1% + 5 + 1, midlen%);
    LET rmid$ = REFLECT$(mid$);
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: WHAT MAKES YOU THINK I ", rmid$, " YOU?";
    IF i% == 2 THEN PRINT "ELIZA: WHY DO YOU BELIEVE I ", rmid$, " YOU?";
    LET responded% = 1;
  END

  REM 5) * MY *
  LET k$ = " MY ";
  LET p% = INSTR(s$, k$);
  IF p% > 0 AND responded% == 0 THEN BEGIN
    LET x$ = MID$(s$, p% + LEN(k$) + 1);
    LET rx$ = REFLECT$(x$);
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: TELL ME MORE ABOUT YOUR ", rx$, ".";
    IF i% == 2 THEN PRINT "ELIZA: IS IT IMPORTANT TO YOU?";
    LET responded% = 1;
  END

  REM 6) * BECAUSE *
  IF INSTR(s$, " BECAUSE ") > 0 AND responded% == 0 THEN BEGIN
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: IS THAT THE REAL REASON?";
    IF i% == 2 THEN PRINT "ELIZA: DOES THAT EXPLAIN ANYTHING ELSE?";
    LET responded% = 1;
  END

  REM 7) * FAMILY * or specific family members
  IF responded% == 0 THEN BEGIN
    IF INSTR(s$, " FAMILY ") > 0 THEN PRINT "ELIZA: TELL ME MORE ABOUT YOUR FAMILY."; ELSE
    IF INSTR(s$, " MOTHER ") > 0 THEN BEGIN
      LET i% = NEXT%(2);
      IF i% == 1 THEN PRINT "ELIZA: TELL ME MORE ABOUT YOUR MOTHER."; ELSE PRINT "ELIZA: DOES YOUR MOTHER INFLUENCE YOU MUCH?";
      LET responded% = 1;
    END ELSE IF INSTR(s$, " FATHER ") > 0 THEN BEGIN
      LET i% = NEXT%(2);
      IF i% == 1 THEN PRINT "ELIZA: TELL ME MORE ABOUT YOUR FATHER."; ELSE PRINT "ELIZA: DO YOU GET ALONG WITH YOUR FATHER?";
      LET responded% = 1;
    END
  END

  REM 8) * DREAM *
  LET p% = INSTR(s$, " DREAM ");
  IF p% > 0 AND responded% == 0 THEN BEGIN
    LET x$ = MID$(s$, p% + LEN(" DREAM ") + 1);
    LET rx$ = REFLECT$(x$);
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: DO YOU OFTEN DREAM ", rx$, "?"; ELSE PRINT "ELIZA: WHAT DO YOU SUPPOSE THE DREAM MEANS?";
    LET responded% = 1;
  END

  REM 9) * FRIEND *
  IF INSTR(s$, " FRIEND ") > 0 AND responded% == 0 THEN BEGIN
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: TELL ME ABOUT YOUR FRIENDS."; ELSE PRINT "ELIZA: WHY DO YOU MENTION FRIENDS?";
    LET responded% = 1;
  END

  REM 10) * YES *
  IF INSTR(s$, " YES ") > 0 AND responded% == 0 THEN BEGIN
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: I SEE."; ELSE PRINT "ELIZA: PLEASE GO ON.";
    LET responded% = 1;
  END

  REM 11) * NO *
  IF INSTR(s$, " NO ") > 0 AND responded% == 0 THEN BEGIN
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: ARE YOU SAYING NO JUST TO BE NEGATIVE?"; ELSE PRINT "ELIZA: WHY NOT?";
    LET responded% = 1;
  END

  REM 12) * COMPUTER *
  IF INSTR(s$, " COMPUTER ") > 0 AND responded% == 0 THEN BEGIN
    LET i% = NEXT%(2);
    IF i% == 1 THEN PRINT "ELIZA: DO COMPUTERS WORRY YOU?"; ELSE PRINT "ELIZA: DO YOU THINK MACHINES CAN THINK?";
    LET responded% = 1;
  END

  IF responded% == 0 THEN DEFAULT_REPLY();
END

