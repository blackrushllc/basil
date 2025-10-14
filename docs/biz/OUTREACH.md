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



