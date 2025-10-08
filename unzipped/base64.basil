REM Base64 demo for Basil
REM How to run with feature enabled (PowerShell/Windows):
REM   cargo run -q -p basilc --features obj-base64 -- run examples\base64.basil

PRINTLN "Base64 demo";
LET a$ = "Hello, World!";
LET encoded$ = BASE64_ENCODE$(a$);
LET decoded$ = BASE64_DECODE$(encoded$);
PRINTLN "Original: ", a$;
PRINTLN "Encoded : ", encoded$;
PRINTLN "Decoded : ", decoded$;
