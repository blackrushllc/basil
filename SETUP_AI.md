# Quickstart: Basil AI (obj-ai)

Follow these steps to get the AI helpers working and try the demos.


1) Set your API key environment variable (See lower down in thei setup guide for more info on getting an API key)

- Windows (PowerShell):
  - $Env:OPENAI_API_KEY = "sk-..."
- macOS/Linux (bash/zsh):
  - export OPENAI_API_KEY="sk-..."

2) (Optional) Create a .basil-ai.toml with sensible defaults

Create this file at your project root:

```
api_key = "env:OPENAI_API_KEY"
default_model = "gpt-4o-mini"
temperature = 0.3
max_tokens = 400
cache = true
```

3) Build with the AI feature

- cargo build -p basilc --features obj-ai

4) Run the demos

- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\01_hello_ai.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\02_stream_joke.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\03_explain_file.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\04_embeddings_search.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\05_moderation_gate.basil

5) Try the AI REPL (streaming chat)

- cargo run -q -p basilc --features obj-ai -- --ai

6) Offline/test mode (no network required)

- Either run through the test harness:  
  cargo run -q -p basilc --features obj-ai -- test examples\obj-ai\01_hello_ai.basil
- Or set TEST_MODE=1 in your environment before running.

Notes

- If no API key is found and you’re not in test mode, AI calls fail gracefully: you’ll get empty strings/vectors and AI.LAST_ERROR$ = "missing API key".
- Never print or log your API key. The library avoids echoing secrets in errors.


# Addemdum - Getting your API key:

Here’s a concise instruction section you can drop directly into `SETUP_AI.md` in your **Basil** repo:

---

## Getting Your OpenAI API Key

Basil’s AI features (like `AI.CHAT$`, `AI.STREAM`, `AI.EMBED`, and `AI.MODERATE%`) require an **OpenAI API key** to connect to the OpenAI service.

Follow these steps to get and set up your key:

### 1. Create an OpenAI Account

If you don’t already have one, go to [https://platform.openai.com/signup](https://platform.openai.com/signup) and create an account.

### 2. Get Your API Key

Once logged in, visit the **API Keys** page:
👉 [https://platform.openai.com/api-keys](https://platform.openai.com/api-keys)

Click **“Create new secret key”**, give it a name like “Basil”, and copy the key that begins with `sk-...`.

> ⚠️ Keep your key secret — do not share it or commit it to GitHub.

### 3. Set the Environment Variable

Basil looks for your key in an environment variable called `OPENAI_API_KEY`.

On **macOS/Linux**, open a terminal and run:

```bash
export OPENAI_API_KEY="sk-yourkeyhere"
```

To make this permanent, add that line to your `~/.bashrc`, `~/.zshrc`, or profile file.

On **Windows PowerShell**, run:

```powershell
setx OPENAI_API_KEY "sk-yourkeyhere"
```
Or using the Windows GUI do this:

1. Open the Windows Control Panel
2. Click System and Security
3. Click System
4. Click Advanced system settings
5. Click Environment Variables
6. Click New...
7. For Variable name, enter `OPENAI_API_KEY`
8. For Variable value, enter `sk-yourkeyhere`
9. Click OK
10. Click OK

### 4. Test It in Basil

Run:

```bash
 cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\01_hello_ai.basil
 cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\02_stream_joke.basil
 cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\03_explain_file.basil
 cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\04_embeddings_search.basil
 cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\05_moderation_gate.basil
```

If everything is working, Basil should print successful responses from each AI function.

---
