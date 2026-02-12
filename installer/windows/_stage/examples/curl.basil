REM CURL demo using HTTP_GET$ and HTTP_POST$ (requires building with --features obj-curl)

PRINTLN "CURL demo"

LET url$ = "https://yobasic.com/basil/hello.basil"
PRINTLN "GET ", url$

LET body$ = HTTP_GET$(url$)
PRINTLN "Response length: ", LEN(body$)
PRINTLN "Preview (first 120 chars):"
PRINTLN LEFT$(body$, 120)

// Optional: print full body
// PRINTLN body$

//' Simple POST demo (echo service)

LET post_url$ = "https://httpbin.org/post"
LET json$ = "{\"hello\": \"world\", \"number\": 123}" ' Yes, this is how to raw dog a JSON string in BASIL
LET resp$ = HTTP_POST$(post_url$, json$, "application/json")
PRINTLN "POST to httpbin length: ", LEN(resp$)

PRINTLN "Done."