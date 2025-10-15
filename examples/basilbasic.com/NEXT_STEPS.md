# Project layout

```
/ (root)
  index.basil
  login.basil
  logout.basil
  register.basil
  user_home.basil
  /css/site.css
  /js/site.js
  /views/
    home.html
    login.html
    register.html
    logged_in.html
```

# What’s in the views (quick check)

* `views/home.html` — intro section explaining the tiny Basil CGI skeleton and SQLite user store.
* `views/login.html` — a POST form to `login.basil` with required username/password fields.
* `views/register.html` — a POST form to `register.basil` with password confirmation.
* `views/logged_in.html` — dashboard stub shown after successful login/cookie set.

# Assets sanity check

* `css/site.css` defines your design tokens and utility classes (buttons, cards, inputs, notices).
* `js/site.js` currently just logs that the skeleton loaded (nice heartbeat).

# Next steps (pick any/all)

1. Confirm your Basil pages are reading the view snippets with `READFILE$("views/…")` and that headers are emitted before any body (CGI rules).
2. If you want, I can:

    * Add a tiny **layout** helper (header/footer shell) and swap the `PRINT READFILE$()` calls into a `render(view$)` function.
    * Wire a super-simple **router** (e.g., `index.basil` dispatches to views by `GET["p"]`) so you can keep a single entrypoint.
    * Flesh out the **auth flow** to hash passwords, set cookie attributes (HttpOnly/SameSite), and add a logout confirmation page.
    * Drop in a **flash message** helper (errors/notices) that your forms can show inline.




