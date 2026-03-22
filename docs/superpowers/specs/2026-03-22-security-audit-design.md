# Security Audit Design — Careless Whisper
**Date:** 2026-03-22
**Author:** Brainstorming session with Claude Code
**Status:** Approved

## Overview

A one-time security audit of the Careless Whisper codebase covering two layers:

1. **Dependency audit** — automated tools check every Rust and npm package against public vulnerability databases (NVD). Flagged packages include severity, description, and recommended fix (typically a version upgrade).

2. **Static code review** — manual examination of the app's own source code for specific vulnerability classes.

## Goals

- Harden the user's personal fork before customization
- Surface findings suitable for upstream contribution
- Produce a beginner-friendly findings report with plain-English explanations

## Vulnerability Classes in Scope

| Class | Description |
|-------|-------------|
| Command injection | Linux paste code shells out to `xdotool`/`wtype`/`ydotool`; check whether transcribed text or other user-controlled strings reach shell commands |
| IPC input validation | Tauri commands (`download_model`, `delete_model`, `update_settings`) accept strings from the frontend — check for path traversal (e.g. `../../`) and URL injection (`download_model` uses the same `name` string to build both the download URL and the local file path) |
| CSP disabled | `"csp": null` in `tauri.conf.json` disables the webview Content Security Policy, leaving it open to XSS if user content is ever rendered as HTML; also assess `get_recent_logs` which returns raw log text (including transcriptions) to the webview |
| Unsafe Rust | Any `unsafe {}` blocks that could allow memory corruption |
| Overly broad permissions | App requests more OS-level access than needed (microphone, accessibility, autostart); also check `macOSPrivateApi: true` in `tauri.conf.json` and whether it is justified |
| Secrets / credentials | Hardcoded tokens, API keys, or sensitive URLs in the codebase |
| Settings persistence | Config file written/read safely, no TOCTOU or injection issues |
| Model download | Assess whether integrity checking (hash/signature) is absent on downloaded `.bin` before it is loaded into whisper.cpp — given the fixed Hugging Face source URL |
| Local IPC / FIFO abuse | Linux named pipe at a user-writable path (`~/.local/share/careless-whisper/careless-whisper.sock`) accepts commands that trigger recording without hotkey interaction — any process running as the same user can write to it |

## Files / Areas Examined

| Area | Files |
|------|-------|
| Dependency audit | `Cargo.toml`, `package.json`, `Cargo.lock` |
| Linux shell-out | `src-tauri/src/output/paste.rs` |
| IPC commands | `src-tauri/src/commands.rs` |
| CSP config | `src-tauri/tauri.conf.json` |
| Unsafe Rust | All `.rs` files |
| Permissions | `src-tauri/Info.plist`, `src-tauri/tauri.conf.json` (incl. `macOSPrivateApi: true`), `src-tauri/src/lib.rs` |
| Secrets scan | Entire repository |
| Settings persistence | `src-tauri/src/config/settings.rs` |
| Model download | `src-tauri/src/models/downloader.rs` |
| Linux FIFO listener | `src-tauri/src/lib.rs` (lines 78–146) |

## Out of Scope

- whisper.cpp C++ library internals
- OS-level sandbox escapes
- Network traffic analysis
- Dynamic / runtime testing (future Option C pass)

## Note on whisper.cpp

This app uses **whisper.cpp** (by Georgi Gerganov, `ggerganov` on GitHub) via the `whisper-rs` Rust crate (v0.12.0), **not** OpenAI's Python Whisper library. whisper.cpp has its own CVE history; the audit will check `whisper-rs` 0.12.0 against known advisories.

## Output

A findings report saved to `docs/security/audit-2026-03-22.md` with:

- Findings organized by severity: Critical → High → Medium → Low → Informational
- Each finding includes:
  - Affected file and line number
  - Plain-English explanation of the vulnerability
  - Concrete exploitation scenario
  - Suggested fix with before/after code snippets where helpful
- Summary table of all findings
- Recommended fix order

### Severity Levels

| Level | Meaning |
|-------|---------|
| **Critical** | Can be exploited remotely or with minimal user interaction to run code or steal data |
| **High** | Requires local access or specific conditions, but impact is severe |
| **Medium** | Limited impact or hard to exploit, but worth fixing |
| **Low** | Best-practice issue, low real-world risk |
| **Info** | Not a vulnerability — just something worth knowing |

## Approach Selected

**Option B — Automated audit + static code review.** Chosen over Option C (dynamic testing) because it finds the highest-value issues without requiring a working build. Dynamic testing is recommended as a follow-up second pass after findings are addressed.
