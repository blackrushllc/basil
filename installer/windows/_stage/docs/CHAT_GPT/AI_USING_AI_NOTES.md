# Repo Review & Hand-Off Templates (Basil/Yore)

## Just an observaion:

> Junie Ultimate is fantastic at carrying out really complex prompts and enacting massive code changes to a complicated project.
> GitHub co-pilot is like an intern who can do simple things one at a time but should probably hope it's looks are better
> than it's brains if it hopes to be successful in life

### Chat GPT's opinion:

Haha, that summary of Copilot made me snort. 😄

Your division of labor is spot-on. If you ever want to get the most out of this combo, here’s a quick playbook:

**How to use me (fast, accurate context):**

* Send exact URLs (or permalinks to a commit) and what you want: “Summarize README + BASIL_CGI.md,” “map architecture,” “explain X module,” “spot breaking changes since v0.3,” etc.
* If it’s code-level, include file paths + line ranges (or a diff). I’ll cite and give concrete next steps/prompts for Junie.

**How to hand off to Junie (surgical edits at scale):**

* Give Junie a clean, numbered task list I can draft: migrations to run, files to edit, acceptance tests to add, and a rollback plan.
* Ask Junie to open PRs with commit scopes that match the task list. (I can generate the PR descriptions/checklists.)

**Copilot sweet spot (quick assists):**

* Inline refactors, boilerplate, converting small snippets/tests, or filling out repetitive code patterns you’ve already designed.

If you want, I can spin up a reusable “Repo Review” prompt you can paste here anytime. Something like:

> **Repo review template**
>
> * Repo/branch/commit: <link>
> * Scope: (README + docs | specific module | end-to-end)
> * Questions: (e.g., risks, missing tests, unclear contracts)
> * Deliverables: (bullet summary, architecture diagram, Junie task list, PR template)


> Yeah! These notes are a great addition to the project!



# Repo Review & Hand‑Off Templates

These are copy‑paste templates you can reuse for any project. They’re tuned for your Basil/Yore workflow with Junie Ultimate and (optionally) Copilot.

---

## 1) Repo Review Intake

```
**Repo/Branch/Commit:** <URL to repo + branch> | <permalink to commit/diff>
**Scope:** (README + docs | specific module | end‑to‑end)
**Primary Goals:** <e.g., summarize, identify risks, map architecture, generate prompts>
**Key Questions:** <bulleted list>
**Constraints:** <perf, security, licensing, deadlines>
**Artifacts Wanted:** (bullet digest | architecture sketch | change log | Junie task list | PR template)
```

### Helpful Links

```
- README: <URL>
- Docs: <URL>
- CI/CD: <URL>
- Issues/Board: <URL>
- Releases/Tags: <URL>
```

---

## 2) Quick Skim Summary (what I’ll return to you)

```
**What it is:** <one‑liner>
**Why it exists:** <problem solved/users>
**How it works (10‑line tour):**
1. <entry points>
2. <runtime/targets>
3. <modules/dirs>
4. <notable deps>
5. <IO/contracts>
6. <build/feature flags>
7. <testing>
8. <CI/CD>
9. <packaging>
10. <docs/resources>
**Gotchas/Risks:** <bullets>
**Next steps:** <bullets>
```

---

## 3) Architecture Map (lightweight)

```
**Entry Points:** <binaries, scripts, CGI endpoints>
**Core Modules:** <module → responsibilities>
**Data:** <schemas/files/queues>
**Boundaries/Interop:** <FFI, WASI, HTTP, CLI>
**Config/Envs:** <feature flags, .env, secrets>
**Error/Observability:** <logging, metrics>
```

---

## 4) Diff/Release Analysis

```
**Range:** <vX.Y.Z..main or commitA..commitB>
**Breaking Changes:** <API/ABI/behavior>
**Feature Flags Affected:** <list>
**Dep Updates:** <major/minor/pins>
**Security/License Notes:** <SBOM, notices>
**Migration Steps:** <ordered>
```

---

## 5) Risk & TODO Matrix

```
| Area | Risk | Impact | Likelihood | Mitigation | Owner |
|------|------|--------|------------|------------|-------|
```

---

## 6) Hand‑Off to Junie (surgical edits at scale)

```
**Context:** <1‑3 lines summary + links>
**Acceptance Tests:**
- [ ] <test case>
- [ ] <test case>
**Tasks:**
1) <file path>: <edit request>
2) <file path>: <edit request>
3) <new file>: <content/outline>
**Non‑Goals:** <explicitly out of scope>
**PR Strategy:** <one PR per area or stacked PRs>
**Rollback Plan:** <how to revert safely>
```

**Commit Message Conventions**

```
<type>(scope): short imperative summary

Why:
- <reason>
Changes:
- <list>
Tests:
- <list>
```

---

## 7) Pull Request Template (drop into `.github/pull_request_template.md`)

```
### Summary
- What changed and why.

### Type
- [ ] Feature  [ ] Fix  [ ] Refactor  [ ] Docs  [ ] Build/CI

### Scope
- Modules touched: <list>
- Feature flags: <on/off>

### Testing
- [ ] Unit  [ ] Integration  [ ] Manual  [ ] e2e

### Risks/Mitigations
- <bullets>

### Screenshots/CLI Output
- <optional>

### Checklist
- [ ] Docs updated
- [ ] Backward compatibility considered
- [ ] Security/PII reviewed
```

---

## 8) Copilot “Intern Tasks” (micro‑asks it’s good at)

```
- Convert snippet X from A→B
- Generate table‑driven tests for function Y
- Extract function Z + add docstring
- Add guard clauses + early returns in file Foo
- Fill in repetitive boilerplate (serde/struct opts, CLI flags)
```

---

## 9) How to Send Me a Great Review Request

```
1) Paste permalinks (press “y” on GitHub to freeze to a commit).
2) Specify scope + artifacts (sections 1–2 above).
3) Mention deadlines/constraints.
4) If you want a Junie hand‑off, say “Generate section 6 for me.”
```
