If you want people in the software / dev-tools / programming media world to look at your BASIC compiler “Basil” (especially in its current form as a GitHub repo / tool for developers), here’s a suggested strategy and some names / types of places to reach out to. You might tailor based on your own domain (embedded, compilers, education, retro computing, etc.).

## Types of people / outlets to target

You’ll want to aim for reviewers / writers / influencers who are comfortable digging into developer tools, programming languages, compilers, open source software, etc. A few categories:

1. **Rust community bloggers / newsletter authors / prominent Rust developers**
   Rust has a vibrant community; many folks blog about new crates, language tools, compiler internals, projects integrating with Rust, etc.
   Examples:

    * The *Rust blog / Inside Rust* (though they are selective) ([Rust Forge][1])
    * Individual Rust bloggers or newsletter authors (e.g. “This Week in Rust”, “Rust Weekly”, etc.)
    * People who do live code reviews or live coding streams of interesting Rust projects

2. **Compiler / PL / language tool bloggers / researchers / associations**
   People writing about domain theory, language design, compilers, interpreters. Even academic or semi-academic blogs.

3. **Developer tooling / open source / systems software media**
   Outlets that cover new open source tools, languages, developer productivity tools, etc. (e.g. Ars Technica’s programming side, InfoQ, The Register, etc.)

4. **YouTubers / Twitch streamers / live coders**
   People who explore new languages, compilers, retro / classic languages and show how to build or use them.

5. **Rust/Rust-adjacent conference demo tracks / workshops**
   Submitting a talk or demo to a Rust or systems conference is another vector for exposure and for people to dig into your tooling.

---

## Steps to approach / submit your request

Here’s a sequence you could follow. You may not do them all, but they help smooth the path.

1. **Polish your GitHub repo for reviewers**
   Before reaching out, make sure your repo is well organized, documented, and “reviewer friendly.” Some things to ensure:

    * A good **README** that outlines what Basil is, what it does, how to build / run / test, what the architecture is, and what use cases it supports.
    * Clear **getting started instructions** (clone, build, run a sample). Even if you don’t have a packaged binary, you should have example code / tests / demos.
    * Highlight “why Basil is interesting / novel” — what makes it stand out vs existing BASIC compilers or interpreter tools. If there are performance, safety, interoperability, or design tradeoffs you’ve addressed, call them out.
    * Some small sample programs or benchmarks or demos that someone can run quickly without too much setup.
    * Maybe tag issues “first-issue” or “help wanted” so people know where to poke.
    * Licensing and contribution guidelines (so reviewers know how open it is).

2. **Prepare a “pitch / outreach package”**
   When contacting a media person, have a concise, friendly pitch that includes:

    * What *Basil* is (short elevator description)
    * Why it’s interesting / novel (what justifies writing about it)
    * Who the intended users are (developers, hobbyists, retro computing folks, etc.)
    * What you want from them (a write-up, a code review, a live stream, a blog post)
    * A link to the GitHub repo, plus instructions (or pointers) to get started
    * Possibly a “getting started video / GIF / screenshot” to whet interest
    * Offer to provide help, support, or a small guided walkthrough if needed
    * Optionally, a “preview / early access / contact” status (e.g. “this is experimental but stable enough for review”)
    * If possible, note dependencies, build requirements, etc., so they know upfront what they’ll need to install.

3. **Identify specific reviewers / media contacts**
   Do some research and compile a list of people / outlets who might have an interest. Some tactics:

    * Search for “Rust tool review”, “new Rust crates blog”, “compiler blog” etc.
    * Look at who is writing about compilers, or retro languages, or languages in general.
    * On Twitter / X, LinkedIn, or Mastodon, search hashtags like `#rust`, `#compilers`, `#programmingtools` — see who posts about new tools / projects.
    * Check past articles: e.g. if someone has reviewed a new compiler or language tool in the last year, they might be open to reviewing yours.
    * Conferences / workshops: find program committee members or speakers in relevant areas and approach them.

4. **Send personalized outreach**
   Don’t mass-spam; try to personalize each message (mention a previous relevant article of theirs, or something in their interests). Email or DM via their preferred channel (some bloggers list contact info). In your message:

    * Introduce yourself and Basil succinctly
    * Explain why you think they would be interested
    * Provide a “try it yourself” link / guide
    * Optionally offer some exclusivity (e.g. “if you want a quick walkthrough or first crack at features X, happy to arrange”)
    * Be polite and open to feedback

5. **Offer support during review**

    * Be responsive: if they run into build issues, dependency problems, or want help understanding your code, assist quickly
    * If they suggest changes or improvements (small ones) before publishing, consider making them or explaining tradeoffs
    * Possibly offer to co-author a blog post or walk through the architecture with them

6. **After review: amplify / follow up**

    * Once someone publishes a blog post, video, or review, share it widely (on Twitter/X, Reddit, Hacker News, Rust community forums, etc.)
    * Thank the reviewer publicly
    * Use feedback to improve Basil, and eventually produce a more polished release / distribution package
    * Keep engaging with the community — reply to comments, fix issues, and iterate.

7. **Later: more formal packaging & releases**
   As you get interest and feedback, plan to produce release artifacts (binaries, crates, installation guides) so that non-developer reviewers / users can try it more easily. That will broaden your reach beyond those comfortable compiling from source.

---

## Some names / places you might try reaching out to

While I don’t have a precise list tuned to Basil, here are some candidate categories / examples:

* **Rust community:**

    * Authors / maintainers of popular Rust blogs / newsletters (e.g. *This Week in Rust*, *Rust Advent*, etc.)
    * People who stream Rust development / tool building
    * Rust Foundation blog editors (for “news about new tools”)
    * Rust subreddit (r/rust) — you could post a “project spotlight / work in progress” after you have something reasonably working

* **Programming languages / compilers scene:**

    * Bloggers or authors who cover compiler design, language tools (e.g. “Lambda the Ultimate” or personal blogs)
    * Academic / educational blogs that teach or review language tools
    * The folks behind “Compiler Explorer” (Godbolt), if relevant
    * Conferences: PLDI, POPL, or smaller language tool workshops (submit a demo or poster)

* **Developer / tech media:**

    * InfoQ
    * The Register
    * Hacker News (submit the project to the “Show HN” section)
    * Ars Technica / IEEE Spectrum (for languages/tools)
    * Languages of interest magazines or blogs

* **YouTube / streaming:**

    * Channels that explore new languages or compilers
    * Live coders who might demo building Basil
    * Retro computing / BASIC revival channels

You can find specific names by looking at recent articles in these spaces, or by searching “review new Rust tool blog” etc.

---

[1]: https://forge.rust-lang.org/platforms/blogs.html?utm_source=chatgpt.com "Blogs"








Awesome—here’s a focused, dev-tool–friendly press + community list you can pitch *today*, plus a tight outreach checklist and a short email template you can paste.

# Who to contact (and why)

**Rust newsletters & community**

1. **This Week in Rust** – weekly newsletter; accepts PRs/mentions & CFP items. Great for early visibility and “call for participation.” ([this-week-in-rust.org][1])
2. **Rust Users Forum** – announcement + community categories for project spotlights and contributor calls. ([The Rust Programming Language Forum][2])
3. **r/rust** (Reddit) – project posts are OK if on-topic and non-spammy; read the rules first. ([Reddit][3])
4. **Rustacean Station (podcast)** – community-run; invites topic/episode pitches ([hello@rustacean-station.org](mailto:hello@rustacean-station.org) / Discord). ([Rustacean Station][4])

**Developer media (news/analysis)**
5) **InfoQ** – dev/architecture audience; accepts contributed articles and pitches (editors@/contribute@ + form). A “building a BASIC compiler in Rust” explainer fits. ([InfoQ][5])
6) **Ars Technica** – tech news with dev readership; use the contact form or press@ for pitches. ([Ars Technica][6])
7) **The Register / DevClass** – enterprise/dev-tool coverage; email [news@theregister.com](mailto:news@theregister.com); DevClass at [devclass@sitpub.com](mailto:devclass@sitpub.com). ([The Register][7])
8) **Hackaday** – loves novel compilers/toolkits; email [tips@hackaday.com](mailto:tips@hackaday.com) or use tip form. ([Hackaday][8])

**Launch hubs / discussion sites**
9) **Hacker News (Show HN)** – post when there’s a clear “try it now” path (clone + cargo build + run). Read guidelines; can even email [hn@ycombinator.com](mailto:hn@ycombinator.com) for posting tips. ([Hacker News][9])
10) **Lobsters (computing community)** – invite-only but high-signal; great for PL/compilers write-ups once you have access. ([GitHub][10])

**Video & streamers who cover Rust/PL**
11) **Jon Gjengset (YouTube/Twitch)** – deep Rust live-coding and language internals; ideal for a walkthrough stream if he’s interested. ([YouTube][11])
12) **Tsoding (Twitch/YouTube)** – frequent language/tool hacking streams; business email listed. ([Twitch][12])
13) **Let’s Get Rusty** – Rust-focused channel; site and contact email visible via course pages. ([Let's Get Rusty][13])

**Extra Rust channels**
14) **This Week in Rust – “Call for Participation”** – submit concrete “good first issues” to attract contributors. ([this-week-in-rust.org][14])
15) **Rust “Call for Participation” thread (Users Forum)** – ongoing place to list tasks/issues. ([The Rust Programming Language Forum][15])

**Bonus (broader dev)**
16) **Show on r/ProgrammingLanguages & r/Compilers** – discussion-oriented audiences; be mindful of self-promo norms. ([Programming Language Design][16])

---

# Quick outreach checklist (1–2 hours of prep)

* **Polish the repo for reviewers**

    * Top-tier README (what/why/how), quickstart (`git clone …; cargo build; cargo run examples/…`), and a *1-minute* demo script.
    * **Samples**: include 2–3 tiny `.basil` programs and a GIF or asciicinema of Basil compiling/running.
    * **Issues**: tag a few as `good first issue` and one `Call for Participation` item to link in pitches (helps for TWiR + forums). ([this-week-in-rust.org][14])

* **Create a mini press kit**

    * 2–3 screenshots/GIFs, one-page feature overview, bullets on what’s novel (e.g., Rust VM + bytecode, debugger hooks, AI object lib, etc.).
    * A **“Try it now”** gist section: prerequisites, `cargo` version, build steps, and one command that runs a cool demo.

* **Choose your first 5 targets**

    * Suggestion: This Week in Rust, Hackaday tips, InfoQ (article pitch), r/rust post, and a Show HN when your quickstart is buttery-smooth. ([this-week-in-rust.org][17])

---

# Short pitch email (paste & customize)

Subject: New open-source BASIC compiler in Rust — “Basil” (repo walkthrough + demos)

Hi <Name>,

I’m building **Basil**, a modern BASIC **compiler + VM written in Rust**. It’s open-source and ready for hands-on testing (clone + cargo build). What’s interesting:

* Tight, Rust-native toolchain (lexer → parser → bytecode → VM), fast builds
* Clean standard library + sample programs (graphics/audio demos optional)
* Early debugger hooks (breakpoints, stepping, scopes) and LSP-friendly diagnostics
* Designed for extensibility (e.g., AI and DAW modules planned/available)

**Repo**: <GitHub link>
**1-minute quickstart**: `git clone … && cargo build && cargo run -- examples/hello.basil`
**Press kit**: <link to screenshots/GIFs>
If you’re up for it, I can do a 10-minute guided walkthrough or provide a reviewer branch with extra examples.

Thanks for taking a look,
Erik (Blackrush LLC) <email> • <Mastodon/Twitter> • <optional website>

---

# Posting notes & tips

* **This Week in Rust**: open a PR/mention; if recruiting help, submit specific tasks to the CfP section. ([this-week-in-rust.org][1])
* **Hacker News (Show HN)**: make sure “playable now” is true (clear build/run). Read the Show HN page; you can even email HN for posting tips. ([Hacker News][9])
* **Hackaday**: email the tip line with your repo, a couple of GIFs, and a sentence on what’s novel (e.g., “BASIC compiler with Rust VM + bytecode + debugger hooks”). ([Hackaday][8])
* **InfoQ**: pitch a practitioner article—“Designing a BASIC compiler/VM in Rust: architecture & lessons learned”—via the guidelines/form. ([InfoQ][5])
* **r/rust and forums**: follow the rules; avoid spam; include clear instructions, code snippets, and a request for feedback. ([Reddit][3])

If you want, tell me Basil’s current headline features (and the repo URL), and I’ll draft **customized messages** for 5 of these targets tailored to their audience.

[1]: https://this-week-in-rust.org/blog/2025/10/01/this-week-in-rust-619/?utm_source=chatgpt.com "This Week in Rust 619"
[2]: https://users.rust-lang.org/?utm_source=chatgpt.com "The Rust Programming Language Forum"
[3]: https://www.reddit.com/r/rust/wiki/rules/?utm_source=chatgpt.com "r/rust Rules: Community Guidelines"
[4]: https://rustacean-station.org/?utm_source=chatgpt.com "Rustacean Station"
[5]: https://www.infoq.com/guidelines/?utm_source=chatgpt.com "Author Guidelines for InfoQ Articles"
[6]: https://arstechnica.com/contact-us/?utm_source=chatgpt.com "Contact us"
[7]: https://www.theregister.com/Profile/contact/?utm_source=chatgpt.com "Contact The Register"
[8]: https://hackaday.com/submit-a-tip/?utm_source=chatgpt.com "Submit A Tip"
[9]: https://news.ycombinator.com/showhn.html?utm_source=chatgpt.com "Show HN Guidelines"
[10]: https://github.com/lobsters/lobsters?utm_source=chatgpt.com "lobsters/lobsters: Computing-focused community centered ..."
[11]: https://www.youtube.com/c/JonGjengset?utm_source=chatgpt.com "Jon Gjengset"
[12]: https://www.twitch.tv/tsoding/about?utm_source=chatgpt.com "About Tsoding"
[13]: https://letsgetrusty.com/?utm_source=chatgpt.com "Let's Get Rusty"
[14]: https://this-week-in-rust.org/blog/2025/09/10/this-week-in-rust-616/?utm_source=chatgpt.com "This Week in Rust 616"
[15]: https://users.rust-lang.org/t/call-for-participation/120776?utm_source=chatgpt.com "Call for Participation - help"
[16]: https://proglangdesign.net/wiki/discord?utm_source=chatgpt.com "The /r/ProgrammingLanguages Discord - PLD Wiki"
[17]: https://this-week-in-rust.org/?utm_source=chatgpt.com "This Week in Rust"






