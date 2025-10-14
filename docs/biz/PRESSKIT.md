# 🗞️ Basil Press Kit

> A modern BASIC-flavored language written in Rust — simple, fast, and AI-native.  
> Created by **Erik Olson** · **Blackrush LLC** · Tarpon Springs, Florida  
> License: MIT / Apache-2.0 dual license

---

## 🌿 Overview

**Basil** is a new open-source programming language that re-imagines classic BASIC for the web, backend, and AI era.  
It combines the simplicity of BASIC with the performance and safety of Rust.  
Basil can compile to **native binaries** (Windows, Linux, macOS) or **bytecode** for fast interpreted runs, and it can also act as a **CGI templating engine** using `<?basil ... ?>` tags — just like PHP, but faster.

Basil is **AI-aware** from the ground up: it includes native modules for JSON, SQL, MIDI, AWS, ZIP, Base64, and even AI integration via the `obj-ai` library.  
Its companion tools (`bcc` compiler and `bvm` virtual machine) let developers build, run, and deploy Basil apps across all platforms.

---

## 🧠 Key Features

| Category | Highlights |
|-----------|-------------|
| **Language** | BASIC-style syntax, modern flow control, arrays, classes, modules, and file I/O |
| **Performance** | Rust-based compiler + VM with fast startup and low memory footprint |
| **AI Integration** | Built-in `obj-ai` library for chat, embedding, moderation, and AI-driven code generation |
| **Cross-Platform** | Compile to bytecode or native binaries for Windows, Linux, macOS |
| **Web-Ready** | Run as CGI templating engine with `<?basil ... ?>` tags |
| **Built-In Libraries** | Zip, Base64, JSON, SQL, MIDI, AWS, AI, HTTP, CLI utilities |
| **Creative Tools** | Includes a complete website framework and a MIDI digital audio workstation (DAW) demo |
| **Retro Mode** | Optional GWBASIC-style REPL for nostalgic development |
| **Open Source** | MIT / Apache-2.0 licensed, on GitHub for contributors |

---

## 📸 Media Assets

| Type | Description | File |
|------|--------------|------|
| **Logo (Light)** | Transparent background PNG, green leaf variant | `/docs/media/basil-logo-light.png` |
| **Logo (Dark)** | White text version for dark themes | `/docs/media/basil-logo-dark.png` |
| **Screenshot 1** | Basil REPL showing GWBASIC-style mode | `/docs/media/basil-repl.png` |
| **Screenshot 2** | Basil compiling and running a web app example | `/docs/media/basil-compile.png` |
| **Screenshot 3** | Basil AI module generating code suggestions | `/docs/media/basil-ai.png` |

*(You can replace these with your actual images — the file names are placeholders.)*

---

## 🔗 Quick Links

- **GitHub Repository:** [github.com/blackrushllc/basil](https://github.com/blackrushllc/basil)
- **Reference Manual:** [yobasic.com/basil/reference.html](https://yobasic.com/basil/reference.html)
- **Docs Folder:** [`/docs/`](../docs/) (setup, development notes, examples)
- **License:** [MIT / Apache-2.0](../LICENSE)

---

## 🗣️ Contact

| Role | Contact |
|------|----------|
| Creator | **Erik Olson** |
| Organization | **Blackrush LLC** |
| Location | Tarpon Springs, Florida, USA |
| Email | `press@blackrush.io` *(or your preferred contact)* |
| Website | [yobasic.com](https://yobasic.com) |
| Twitter/X | [@BlackrushLLC](https://twitter.com/BlackrushLLC) |
| Mastodon | [@blackrush@fosstodon.org](https://fosstodon.org/@blackrush) |

---

## 📣 Suggested Mentions

When writing about Basil, please use one of these short descriptions:

**One-liner (general):**
> *Basil is a modern BASIC compiler and VM written in Rust, designed for web, backend, and AI-powered development.*

**For Rust/tech audiences:**
> *Basil brings BASIC’s simplicity to the Rust ecosystem — a safe, fast, AI-integrated language for the modern age.*

**For retro/creative audiences:**
> *Basil revives the joy of BASIC with a bytecode compiler, Rust performance, and AI-driven features — a retro-modern language for today.*

---

## 🕹️ Quick Demo Commands

```bash

git clone https://github.com/blackrushllc/basil.git
cd basil
cargo run -q -p basilc --features obj-all -- run ./examples/hello.basil

````

Or, try the retro REPL:

```bash

cargo run -q -p basilc --features obj-all -- cli

```
---

## 🧩 Tags / Keywords

`#Rust` `#BASIC` `#ProgrammingLanguage` `#Compiler` `#OpenSource`
`#AI` `#Bytecode` `#Interpreter` `#RetroComputing` `#WebDevelopment`

---

*(c) 2025 Blackrush LLC · Created by Erik Olson · MIT / Apache-2.0 Licensed*

