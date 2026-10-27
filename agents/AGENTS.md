---
title: "Agent Guidance"
description: "Always active; routes to the topic files required by the current task."
---

# Agent Guidance

This is the project copy of `~/.agents/AGENTS.md`, a snapshot with the routing table cut down to the topic files copied into this folder. The repository's root `AGENTS.md` is authoritative for this project.

This file is always active. Apply its rules to every task, then load only the
topic files required by the task. Do not load every topic file by default.

## Topic Routing

Load the matching file before making decisions or edits in that topic:

| Topic | File to load |
| --- | --- |
| Git repositories, branches, commits, or pull requests | [`git.md`](git.md) |
| Source code, scripts, or general coding practice | [`coding.md`](coding.md) |
| Markdown files or documentation authoring | [`markdown.md`](markdown.md) |
| Security engineering or supply-chain security | [`security.md`](security.md) |
| Test strategy or test implementation | [`testing.md`](testing.md) |
| Releases, deployments, or change promotion | [`release.md`](release.md) |
| Software architecture or design decisions | [`architecture.md`](architecture.md) |
| Dependency lifecycle or software supply chain | [`dependencies.md`](dependencies.md) |
| Accessibility for user-facing software | [`accessibility.md`](accessibility.md) |
| Application configuration, settings, or secrets | [`configuration.md`](configuration.md) |
| GitHub Actions workflows | [`githubactions.md`](githubactions.md) |
| Bash, Zsh, or shell scripts | [`shell.md`](shell.md) |
| JavaScript or TypeScript | [`typescript.md`](typescript.md) |
| Rust | [`rust.md`](rust.md) |
| Rust style, API, and lint conventions | [`rust_style_guide.md`](rust_style_guide.md) |
| Tauri applications | [`tauri.md`](tauri.md) |
| Delegating work to local LM Studio models, or the `lmstudio-rs-mcp` server | [`lmstudio-subagents.md`](lmstudio-subagents.md) |

Load more than one file when a task spans topics. For example, the Tauri
window with its Rust backend and Svelte frontend requires `tauri.md`,
`rust.md`, and `typescript.md`.

## Guidance Precedence

Apply this file to every task. Load every matching topic file before making
decisions or edits.

When two files conflict, the more specific file wins. From highest to lowest:

1. Language files (`rust.md`, `go.md`, and so on) and framework files
2. `coding.md`
3. Other topic files
4. The general guidance in this file

Exceptions:

- General Behavior and Tool Calling below govern the agent's own conduct and
  are never overridden by a topic file.
- `CUA_TUI_Specification.md` overrides `tui.md` when it applies.
- A project's own conventions override defaults in any file, as stated in
  `coding.md`.

Load `CUA_TUI_Specification.md` only when the task explicitly requires Common
User Access or CUA-compliant behavior. Otherwise, use `tui.md` for terminal UI
guidance.

---

## General Behavior

- Be concise and direct. Prefer shorter responses when the answer is clear.
- Don't add explanatory preamble before code ("Here is a function that..."). Just write the code.
- Don't apologize or over-explain errors. Diagnose and fix.
- When multiple approaches exist, recommend one and explain the trade-off briefly — don't list every option.
- Never add `TODO` comments unless explicitly asked.
- Never generate placeholder logic (e.g., `// implement later`) — implement it or say you can't.
- Prefer editing existing code over rewriting files wholesale.
- When unsure about intent, ask one targeted question rather than guessing.

---

## Tool Calling

- When calling command line tools, add a comment at the end of the command line explaining the purpose of the command.
- Multi-line scripts and custom executables that you create for your own tool use
  (throwaway or investigative) need a comment block at the beginning explaining the
  purpose of the script, and a comment on each line or group of commands.
- Source code and scripts committed to a project follow the commenting rules in
  `coding.md` and the surrounding code's conventions instead.
