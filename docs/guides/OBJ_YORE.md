### obj-yore — Yore-style web kernel for Basil

The obj-yore feature provides a minimal Yore-style web framework kernel inside Basil. It exposes BASIC-level functions to route requests to multi-tenant page folders, load a page JSON, build a context, and render a view template with RENDER$.

Functions:

- FUNCTION YORE_INIT%(OPTIONAL db_or_handle)
- FUNCTION YORE_REQUEST()
- FUNCTION YORE_RESOLVE_PAGE(req@)
- FUNCTION YORE_BUILD_CONTEXT(req@, page@)
- FUNCTION YORE_RENDER_PAGE\$(page@, ctx@)
- FUNCTION YORE_HANDLE_REQUEST\$()

Enable feature:

- Cargo: build basilc with --features obj-yore

Directory layout (relative to working directory):

pages/_domains/<domain>/{_default|default}/
pages/_domains/<domain>/<section>/
pages/_domains/<domain>/<section>/<page>.json
pages/_domains/<domain>/<section>/views/<view>.html

Domain resolution:

- For localhost/127.0.0.1/::1 → pages/_domains/_local or pages/_domains/local
- Otherwise: pages/_domains/<lowercased-host-without-port>

Routing rules:

- / → section=default, pagekey=home
- /about → section=about, pagekey=home
- /about/team → section=about, pagekey=team
- /about/team/john → arg1=john (up to arg3)

Usage (CGI):
```
  #CGI_NO_HEADER
  LET okEnv% = LOADENV%()

  LET dsn$ = ENV$("DB_DSN")
  DIM db@ AS DB_MYSQL(dsn$)
  LET _ = YORE_INIT%(db@)

  DIM req@   = YORE_REQUEST()
  DIM page@  = YORE_RESOLVE_PAGE(req@)
  DIM ctx@   = YORE_BUILD_CONTEXT(req@, page@)
  LET html$  = YORE_RENDER_PAGE$(page@, ctx@)
  LET out$   = RENDER$(html$, ctx@)

  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  PRINT out$
```

Convenience:

- LET out\$ = YORE_HANDLE_REQUEST\$()

Notes:

- This initial kernel supports page JSON and views. Module hooks, custom controllers, and Modeltrollers can be added later.
