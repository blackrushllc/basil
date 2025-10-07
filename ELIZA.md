## 🧠 ELIZA Pattern & Reflection Specification

### 1. Reflection pairs

Use these as the default reflection dictionary for `REFLECT$(s$)`:

| Input    | Output   |
| -------- | -------- |
| I        | YOU      |
| ME       | YOU      |
| MY       | YOUR     |
| MINE     | YOURS    |
| AM       | ARE      |
| I'M      | YOU ARE  |
| YOU      | I        |
| YOUR     | MY       |
| YOURS    | MINE     |
| ARE      | AM       |
| MYSELF   | YOURSELF |
| YOURSELF | MYSELF   |

Junie should normalize both keys and tokens to uppercase when reflecting.

---

### 2. Keyword patterns and example reassemblies

Each **pattern** is checked in priority order.
Each can have several **reassembly templates**, with random choice among them.
Use `<1>`, `<2>`, etc. placeholders for wildcard capture substitution after reflection.

| Pattern        | Example reassemblies                                                                          |
| -------------- | --------------------------------------------------------------------------------------------- |
| `* I FEEL *`   | `WHY DO YOU FEEL <2>?` <br> `DO YOU OFTEN FEEL <2>?` <br> `WHAT MAKES YOU FEEL <2>?`          |
| `* I AM *`     | `HOW LONG HAVE YOU BEEN <2>?` <br> `WHY DO YOU THINK YOU ARE <2>?`                            |
| `* I * YOU *`  | `DO YOU REALLY THINK YOU <2> ME?` <br> `PERHAPS IN YOUR FANTASIES YOU <2> ME.`                |
| `* YOU * ME *` | `WHAT MAKES YOU THINK I <2> YOU?` <br> `WHY DO YOU BELIEVE I <2> YOU?`                        |
| `* MY *`       | `TELL ME MORE ABOUT YOUR <2>.` <br> `IS IT IMPORTANT TO YOU?`                                 |
| `* BECAUSE *`  | `IS THAT THE REAL REASON?` <br> `DOES THAT EXPLAIN ANYTHING ELSE?`                            |
| `* FAMILY *`   | `TELL ME MORE ABOUT YOUR FAMILY.`                                                             |
| `* MOTHER *`   | `TELL ME MORE ABOUT YOUR MOTHER.` <br> `DOES YOUR MOTHER INFLUENCE YOU MUCH?`                 |
| `* FATHER *`   | `TELL ME MORE ABOUT YOUR FATHER.` <br> `DO YOU GET ALONG WITH YOUR FATHER?`                   |
| `* DREAM *`    | `DO YOU OFTEN DREAM <2>?` <br> `WHAT DO YOU SUPPOSE THE DREAM MEANS?`                         |
| `* FRIEND *`   | `TELL ME ABOUT YOUR FRIENDS.` <br> `WHY DO YOU MENTION FRIENDS?`                              |
| `* YES *`      | `I SEE.` <br> `PLEASE GO ON.`                                                                 |
| `* NO *`       | `ARE YOU SAYING NO JUST TO BE NEGATIVE?` <br> `WHY NOT?`                                      |
| `* COMPUTER *` | `DO COMPUTERS WORRY YOU?` <br> `DO YOU THINK MACHINES CAN THINK?`                             |
| `* BYE *`      | `GOODBYE. IT WAS NICE TALKING TO YOU.`                                                        |
| (catch-all)    | `PLEASE TELL ME MORE.` <br> `LET'S TALK MORE ABOUT THAT.` <br> `HOW DOES THAT MAKE YOU FEEL?` |

Junie should store these as an in-memory table (array of objects, arrays, or dicts).
Priority: first match in list wins; pick random reassembly among its set.

---

### 3. Random defaults

Default replies for unmatched inputs:

```
PLEASE TELL ME MORE.
LET'S TALK MORE ABOUT THAT.
HOW DOES THAT MAKE YOU FEEL?
WHY DO YOU SAY THAT?
I SEE. GO ON.
DOES THAT MEAN ANYTHING SPECIAL TO YOU?
CAN YOU ELABORATE?
DOES THIS HAPPEN OFTEN?
WHAT DO YOU THINK?
WHY DO YOU THINK THAT?
INTERESTING. PLEASE CONTINUE.
DO YOU FEEL THAT FREQUENTLY?
```

---

### 4. Example conversation transcript (for validation)

```
ELIZA: HELLO. HOW ARE YOU FEELING TODAY?
YOU: i feel tired
ELIZA: WHY DO YOU FEEL TIRED?
YOU: my job is stressful
ELIZA: TELL ME MORE ABOUT YOUR JOB.
YOU: because my boss yells
ELIZA: IS THAT THE REAL REASON?
YOU: yes
ELIZA: I SEE.
YOU: bye
ELIZA: GOODBYE. IT WAS NICE TALKING TO YOU.
```

---

### 5. Implementation guidance for Junie

* All patterns and reflections should be **case-insensitive**.
* Matching engine supports `*` wildcards; captures accessible via `CAPTURE$(n)`.
* Reflection is done **after** wildcard substitution but before printing.
* Preserve simple punctuation in output (`?` and `.` endings from templates).
* Add randomness via `RND%()` for template selection.


### 6. Original ELIZA source code

````text

10 REM ============================================================
20 REM  ELIZA - Generic BASIC (no line numbers in data; classic flow)
30 REM  Features: normalize, keyword rules, pronoun reflection, random
40 REM  Written by Weizenbaum, 1966
45 REM ============================================================
50 CLEAR
60 DEFINT A-Z
70 DIM RF_IN$(32), RF_OUT$(32)
80 DIM DEF$(16)
90 RANDOMIZE TIMER
100 REM -------- Initialize reflection pairs --------
110 READ RP
120 FOR I = 1 TO RP
130   READ RF_IN$(I), RF_OUT$(I)
140 NEXT I
150 REM -------- Initialize default replies ----------
160 READ DC
170 FOR I = 1 TO DC
180   READ DEF$(I)
190 NEXT I
200 REM -------- Greeting ----------
210 PRINT "ELIZA: HELLO. HOW ARE YOU FEELING TODAY?"
220 REM -------- Main loop ----------
230 INPUTLINE$ = ""
240 PRINT
250 PRINT "YOU: ";
260 LINE INPUT INPUTLINE$
270 IF LEN(INPUTLINE$) = 0 THEN 250
280 GOSUB 5000   ' NORMALIZE: INPUTLINE$ -> NORM$
290 IF NORM$ = "BYE" OR NORM$ = "GOODBYE" OR NORM$ = "QUIT" THEN 9000
300 IF LEN(NORM$) = 0 THEN GOSUB 8000: GOTO 250
310 REM Try rules in priority order
320 S$ = " " + NORM$ + " "
330 REM Rule 1: I FEEL *
340 K$ = " I FEEL "
350 P = INSTR(S$, K$)
360 IF P > 0 THEN X$ = MID$(S$, P + LEN(K$)): GOSUB 6000: GOSUB 7000: PRINT "ELIZA: WHY DO YOU FEEL "; RX$; "?": GOTO 250
370 REM Rule 2: I AM *
380 K$ = " I AM "
390 P = INSTR(S$, K$)
400 IF P > 0 THEN X$ = MID$(S$, P + LEN(K$)): GOSUB 6000: GOSUB 7000: PRINT "ELIZA: HOW LONG HAVE YOU BEEN "; RX$; "?": GOTO 250
410 REM Rule 3: I *
420 K$ = " I "
430 P = INSTR(S$, K$)
440 IF P > 0 THEN X$ = MID$(S$, P + LEN(K$)): GOSUB 6000: GOSUB 7000: PRINT "ELIZA: WHY DO YOU SAY YOU "; RX$; "?": GOTO 250
450 REM Rule 4: MY *
460 K$ = " MY "
470 P = INSTR(S$, K$)
480 IF P > 0 THEN X$ = MID$(S$, P + LEN(K$)): GOSUB 6000: GOSUB 7000: PRINT "ELIZA: TELL ME MORE ABOUT YOUR "; RX$; ".": GOTO 250
490 REM Rule 5: BECAUSE *
500 K$ = " BECAUSE "
510 P = INSTR(S$, K$)
520 IF P > 0 THEN PRINT "ELIZA: IS THAT THE REAL REASON?": GOTO 250
530 REM Rule 6: YOU * ME *
540 K1 = INSTR(S$, " YOU ")
550 K2 = INSTR(S$, " ME ")
560 IF K1 > 0 AND K2 > K1 THEN PRINT "ELIZA: WHAT MAKES YOU THINK I "; MID$(S$, K1 + 5, K2 - (K1 + 5)); " YOU?": GOTO 250
570 REM Rule 7: FAMILY WORDS
580 IF INSTR(S$, " MOTHER ") OR INSTR(S$, " FATHER ") OR INSTR(S$, " FAMILY ") THEN PRINT "ELIZA: TELL ME MORE ABOUT YOUR FAMILY.": GOTO 250
590 REM No match: default
600 GOSUB 8000
610 GOTO 250

4999 REM ================= NORMALIZE ===============================
5000 REM INPUT:  INPUTLINE$
5010 REM OUTPUT: NORM$ (UPPERCASE, PUNCTUATION -> SPACE, TRIM & COLLAPSE SPACES)
5020 T$ = ""
5030 FOR I = 1 TO LEN(INPUTLINE$)
5040   C$ = MID$(INPUTLINE$, I, 1)
5050   A = ASC(C$)
5060   IF A >= 97 AND A <= 122 THEN C$ = CHR$(A - 32): A = ASC(C$)
5070   IF (A >= 65 AND A <= 90) OR (A >= 48 AND A <= 57) OR A = 32 THEN
5080     T$ = T$ + C$
5090   ELSE
5100     T$ = T$ + " "
5110   END IF
5120 NEXT I
5130 REM collapse spaces
5140 U$ = ""
5150 PRV = 1
5160 FOR I = 1 TO LEN(T$)
5170   C$ = MID$(T$, I, 1)
5180   IF C$ = " " THEN
5190     IF PRV = 0 THEN U$ = U$ + " ": PRV = 1
5200   ELSE
5210     U$ = U$ + C$: PRV = 0
5220   END IF
5230 NEXT I
5240 REM trim
5250 WHILE LEN(U$) > 0 AND LEFT$(U$, 1) = " ": U$ = MID$(U$, 2): WEND
5260 WHILE LEN(U$) > 0 AND RIGHT$(U$, 1) = " ": U$ = LEFT$(U$, LEN(U$) - 1): WEND
5270 NORM$ = U$
5280 RETURN

5999 REM ================= REFLECT ================================
6000 REM INPUT:  X$ (fragment after keyword), uses reflection pairs
6010 REM OUTPUT: RX$ (pronoun-reflected string, trimmed)
6020 REM normalize spacing on X$
6030 XS$ = X$
6040 REM trim leading spaces
6050 WHILE LEN(XS$) > 0 AND LEFT$(XS$, 1) = " ": XS$ = MID$(XS$, 2): WEND
6060 REM if empty, return placeholder
6070 IF LEN(XS$) = 0 THEN RX$ = "THAT": RETURN
6080 REM token-by-token replacement
6090 RX$ = ""
6100 W$ = ""
6110 XS$ = XS$ + " "
6120 FOR I = 1 TO LEN(XS$)
6130   C$ = MID$(XS$, I, 1)
6140   IF C$ <> " " THEN
6150     W$ = W$ + C$
6160   ELSE
6170     IF LEN(W$) > 0 THEN
6180       REM look up W$ in reflection pairs
6190       FOUND = 0
6200       FOR J = 1 TO RP
6210         IF W$ = RF_IN$(J) THEN
6220           RX$ = RX$ + RF_OUT$(J) + " "
6230           FOUND = 1
6240           J = RP
6250         END IF
6260       NEXT J
6270       IF FOUND = 0 THEN RX$ = RX$ + W$ + " "
6280       W$ = ""
6290     END IF
6300   END IF
6310 NEXT I
6320 REM trim trailing space
6330 IF LEN(RX$) > 0 AND RIGHT$(RX$, 1) = " " THEN RX$ = LEFT$(RX$, LEN(RX$) - 1)
6340 RETURN

6999 REM ================= SAFE PLACEHOLDER FOR EMPTY X$ ==========
7000 IF LEN(X$) = 0 THEN RX$ = "THAT": RETURN
7010 RETURN

7999 REM ================= DEFAULT REPLY ==========================
8000 R = INT(RND * DC) + 1
8010 PRINT "ELIZA: "; DEF$(R)
8020 RETURN

8999 REM ================== GOODBYE ===============================
9000 PRINT "ELIZA: GOODBYE. IT WAS NICE TALKING TO YOU."
9010 END

9999 REM ================== DATA SECTION ==========================
10000 REM Reflection pairs (count, then pairs)
10010 DATA 12
10020 DATA I,YOU
10030 DATA ME,YOU
10040 DATA MY,YOUR
10050 DATA MINE,YOURS
10060 DATA AM,ARE
10070 DATA IM,YOU ARE
10080 DATA YOU,I
10090 DATA YOUR,MY
10100 DATA YOURS,MINE
10110 DATA ARE,AM
10120 DATA MYSELF,YOURSELF
10130 DATA YOURSELF,MYSELF
10140 REM Default replies (count, then lines)
10150 DATA 12
10160 DATA PLEASE TELL ME MORE.
10170 DATA LET'S TALK MORE ABOUT THAT.
10180 DATA HOW DOES THAT MAKE YOU FEEL?
10190 DATA WHY DO YOU SAY THAT?
10200 DATA I SEE. GO ON.
10210 DATA DOES THAT MEAN ANYTHING SPECIAL TO YOU?
10220 DATA CAN YOU ELABORATE?
10230 DATA DOES THIS HAPPEN OFTEN?
10240 DATA WHAT DO YOU THINK?
10250 DATA WHY DO YOU THINK THAT?
10260 DATA INTERESTING. PLEASE CONTINUE.
10270 DATA DO YOU FEEL THAT FREQUENTLY?

````