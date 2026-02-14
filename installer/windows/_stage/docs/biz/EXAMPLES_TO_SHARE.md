Below are **10 short, “shareable” Basil programs** (each **< 30 lines**, with comments). The first 5 are **core Basil**. The last 5 show **extended “mods”** (AI/SQL/REST/SFTP/SMTP). The repo’s README calls out these mods (AI, REST/CURL, SQL, SFTP, SMTP, etc.). ([GitHub][1])

> Notes:
>
> * I’m writing these in the **classic BASIC-ish Basil style** (LET/PRINTLN/BEGIN…END), since that’s great for social posts.
> * **Some object/function names for the “mods” may vary** depending on which features you compiled in (e.g., `--features obj-ai obj-curl obj-sql ...`). The **AI call `AI.CHAT$()` is confirmed** in Basil’s AI setup docs/article. ([Medium][2])

---

## 1) Core: Dice-roller “loot box”

```basic
REM Dice Loot Box (loops + RANDOM + SELECT CASE)
LET loot$ = ""
FOR i% = 1 TO 5
  LET roll% = 1 + INT(RND() * 6)
  SELECT CASE roll%
    CASE 1: LET loot$ = loot$ + "🧱 "
    CASE 2: LET loot$ = loot$ + "🪙 "
    CASE 3: LET loot$ = loot$ + "🧪 "
    CASE 4: LET loot$ = loot$ + "🗡️ "
    CASE 5: LET loot$ = loot$ + "🛡️ "
    CASE 6: LET loot$ = loot$ + "👑 "
  END SELECT
NEXT i%
PRINTLN "You opened a loot box:", loot$
```

## 2) Core: Tiny “procedural poem” (lists + loops)

```basic
REM Procedural Poem (lists + random picks)
LET adj@ = LIST(); LET noun@ = LIST(); LET verb@ = LIST()
PUSH adj@, "silent": PUSH adj@, "electric": PUSH adj@, "ancient"
PUSH noun@, "ocean":  PUSH noun@, "starship":  PUSH noun@, "library"
PUSH verb@, "dreams": PUSH verb@, "compiles":  PUSH verb@, "whispers"

FOR i% = 1 TO 3
  LET a$ = adj@[ INT(RND()*LEN(adj@)) ]
  LET n$ = noun@[ INT(RND()*LEN(noun@)) ]
  LET v$ = verb@[ INT(RND()*LEN(verb@)) ]
  PRINTLN "The ", a$, " ", n$, " ", v$, "."
NEXT i%
```

## 3) Core: Word-frequency counter (dictionary + foreach)

```basic
REM Word Frequency (dictionary + split + foreach)
LET text$ = "basil basil code code code magic"
LET words@ = SPLIT$(text$, " ")

LET freq@ = DICT()   REM dictionary
FOR EACH w$ IN words@
  IF NOT HASKEY(freq@, w$) THEN freq@[w$] = 0
  freq@[w$] = freq@[w$] + 1
END FOR

PRINTLN "Frequency:"
FOR EACH key$ IN KEYS(freq@)
  PRINTLN " - ", key$, ": ", freq@[key$]
END FOR
```

## 4) Core: “Choose-your-training” mini game (SELECT CASE)

```basic
REM Choose-your-training (input + select case)
PRINTLN "Pick a path: (1) Wizard  (2) Hacker  (3) Captain"
LET c$ = INPUTC$("Choice: ")

SELECT CASE c$
  CASE "1": PRINTLN "You learn SPELL$() and summon a variable."
  CASE "2": PRINTLN "You learn IF/THEN and break reality into functions."
  CASE "3": PRINTLN "You learn LOOPS and command a fleet of processes."
  CASE ELSE: PRINTLN "You stand still... and time loops anyway."
END SELECT
```

## 5) Core: Fibonacci as a reusable function (FUNCTION + loop)

```basic
REM Fibonacci (function + loop)
FUNCTION Fib%(n%)
  IF n% <= 1 THEN RETURN n%
  LET a% = 0: LET b% = 1
  FOR i% = 2 TO n%
    LET t% = a% + b%
    LET a% = b%: LET b% = t%
  NEXT i%
  RETURN b%
END FUNCTION

FOR i% = 0 TO 10
  PRINTLN "Fib(", i%, ") = ", Fib%(i%)
NEXT i%
```

---

## 6) Mods: AI fortune-cookie generator (ChatGPT)

```basic
REM Requires obj-ai + OPENAI_API_KEY
REM Confirmed API style: AI.CHAT$("<prompt>") :contentReference[oaicite:2]{index=2}
LET topic$ = INPUT$("Fortune topic (e.g., coding, space, love): ")
LET prompt$ = "Write ONE funny fortune cookie line about " + topic$ + " (no quotes)."
LET fortune$ = AI.CHAT$(prompt$)
PRINTLN "🥠 ", fortune$
```

## 7) Mods: SQL “tiny leaderboard” (SQLite-style)

```basic
REM Requires SQL/SQLite mod (obj-sql / obj-sqlite)
REM Creates a tiny leaderboard and prints top scores.
LET db@ = SQL.OPEN("sqlite:leaderboard.db")
SQL.EXEC db@, "CREATE TABLE IF NOT EXISTS scores(name TEXT, score INT);"
SQL.EXEC db@, "INSERT INTO scores VALUES('Ada', 120),('Linus', 99),('Basil', 150);"

LET rows@ = SQL.QUERY(db@, "SELECT name, score FROM scores ORDER BY score DESC LIMIT 3;")
PRINTLN "🏆 Top 3:"
FOR EACH r@ IN rows@
  PRINTLN " - ", r@["name"], " : ", r@["score"]
END FOR
SQL.CLOSE db@
```

## 8) Mods: REST “status ping” (GET JSON)

```basic
REM Requires REST/CURL/HTTP mod (often obj-curl)
REM Hits a JSON endpoint and prints one field.
LET url$ = "https://api.github.com/repos/blackrushllc/basil"
LET json$ = HTTP.GET$(url$, {"User-Agent":"BasilDemo"})
LET obj@  = JSON.PARSE(json$)
PRINTLN "Repo: ", obj@["full_name"]
PRINTLN "Stars: ", obj@["stargazers_count"]
```

## 9) Mods: SFTP “grab a remote file” (download)

```basic
REM Requires SFTP mod (obj-sftp)
REM Downloads /remote/hello.txt to ./hello.txt
LET host$ = "sftp.example.com"
LET user$ = INPUT$("User: ")
LET pass$ = INPUT$("Pass: ")

LET s@ = SFTP.CONNECT(host$, user$, pass$, 22)
SFTP.GET s@, "/remote/hello.txt", "./hello.txt"
SFTP.CLOSE s@
PRINTLN "✅ Downloaded hello.txt"
```

## 10) Mods: SMTP “send a one-line email” (automation vibe)

```basic
REM Requires SMTP mod (obj-smtp)
REM Sends a simple message (use app passwords / proper auth)
LET smtp$ = "smtp.example.com"
LET from$ = INPUT$("From: ")
LET to$   = INPUT$("To: ")

LET c@ = SMTP.CONNECT(smtp$, 587)
SMTP.STARTTLS c@
SMTP.AUTH c@, from$, INPUT$("Password: ")

SMTP.SEND c@, from$, to$, "Basil says hi",
  "Hello!\n\nThis email was sent by a 20-line Basil script.\n\n- Basil"
SMTP.QUIT c@
PRINTLN "📨 Sent."
```

If you want, I can also **format these as 10 square “code cards”** (each with a catchy title + one-sentence hook) so they’re ready to paste into X/Facebook/Instagram posts.

[1]: https://github.com/blackrushllc/basil "GitHub - blackrushllc/basil: A Modern, Mod-able, AI-aware, OO (or not) BASIC language Bytecode Interpreter and **Cross-Platform Compiler** with lots of Rad Mods like AI, AWS, Zip, Crypt (Base64, PGP) CrossTerm, Inet (SMTP, FTP, Json, Curl, REST, etc), SQL(MySQL/RDS, Sqlite, ORM, etc), MIDI (Audio, DAW) and even a Totally Tubular \"OK Prompt\" CLI mode (Jolt Cola not included)"
[2]: https://medium.com/%40blackrushdrive/how-to-set-up-ai-in-basil-1409aa7aa09b?utm_source=chatgpt.com "How to Set Up AI in Basil"
