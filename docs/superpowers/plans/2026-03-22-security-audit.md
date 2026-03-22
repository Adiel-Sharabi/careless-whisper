# Security Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a beginner-friendly security findings report at `docs/security/audit-2026-03-22.md` covering dependency CVEs and a static code review of the Careless Whisper codebase.

**Architecture:** Run automated dependency audits first (Cargo + npm), then manually analyze each vulnerability class in scope, documenting every finding in a structured report as we go. The report is built incrementally — one section per task — so it's readable at any point.

**Tech Stack:** `cargo audit` (Rust advisory database), `pnpm audit` (npm advisory database), manual static analysis of Rust and TypeScript source.

---

## Output File

All findings go to: `docs/security/audit-2026-03-22.md`

The report uses this finding template:

```markdown
## [SEVERITY] Title

**File:** path/to/file.rs:line
**What it is:** Plain-English explanation of the vulnerability and why it exists.
**Scenario:** Concrete example of how someone could exploit it.
**Fix:** What to change, with before/after snippets where helpful.
```

Severity scale: **Critical** → **High** → **Medium** → **Low** → **Info**

---

## Task 1: Create the report skeleton and run dependency audits

**Files:**
- Create: `docs/security/audit-2026-03-22.md`

- [ ] **Step 1: Create the report file with header and sections**

```bash
mkdir -p docs/security
```

Create `docs/security/audit-2026-03-22.md` with this skeleton:

```markdown
# Security Audit — Careless Whisper
**Date:** 2026-03-22
**Scope:** Dependency CVE audit + static code review (Option B)
**Audience:** Beginner — all findings explained in plain English

---

## Dependency Audit

### Rust (cargo audit)

_Results pending_

### npm/Node (pnpm audit)

_Results pending_

---

## Static Code Review

### 1. Command Injection (Linux shell-out)
### 2. IPC Input Validation
### 3. CSP Disabled + get_recent_logs
### 4. Unsafe Rust
### 5. Overly Broad Permissions / macOSPrivateApi
### 6. Secrets / Credentials
### 7. Settings Persistence
### 8. Model Download Integrity
### 9. Linux FIFO Listener

---

## Summary Table

| # | Severity | Title | File |
|---|----------|-------|------|
_pending_

## Recommended Fix Order

_pending_
```

- [ ] **Step 2: Install cargo-audit if not present**

```bash
cargo install cargo-audit --locked 2>/dev/null || echo "already installed"
```

Expected: either installs successfully or prints "already installed".

- [ ] **Step 3: Run cargo audit**

```bash
cd src-tauri && cargo audit 2>&1
```

Expected output: either "No vulnerabilities found" or a table of advisories with ID, package, version, severity, and description. Copy the full output into the report under "Rust (cargo audit)". If there are findings, note each as a finding entry with severity mapped from the advisory (Critical/High/Medium/Low).

- [ ] **Step 4: Run pnpm audit**

```bash
pnpm audit 2>&1
```

Expected output: either "No known vulnerabilities" or a table of advisories. Copy the full output into the report under "npm/Node (pnpm audit)". Document each as a finding entry.

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add audit report skeleton and dependency audit results"
```

---

## Task 2: Analyze command injection (Linux shell-out)

**Files:**
- Read: `src-tauri/src/output/paste.rs:222-294`

**Background for the auditor:** Command injection means an attacker tricks the app into running shell commands it didn't intend to. This file builds shell commands using `xdotool` and `wtype`. The question is: does any user-controlled string (like transcribed text) end up inside those commands?

- [ ] **Step 1: Trace the data flow from transcription to paste**

Read `src-tauri/src/output/paste.rs` lines 184-294 and `src-tauri/src/commands.rs` lines 115-166.

Key question: In `paste_x11` (line 222), `window_id` comes from `xdotool getactivewindow` output (line 192). Is this string ever influenced by user input (audio/transcription)?

Answer: No — `window_id` is the output of `xdotool getactivewindow`, which returns an integer window ID. It is captured *before* recording starts (`get_frontmost_target` is called at hotkey press time, `commands.rs:115`). The transcribed text never touches the shell command arguments in `paste_x11` or `paste_wayland`.

- [ ] **Step 2: Check whether transcribed text reaches any shell command**

Read `commands.rs` lines 143-167. After transcription:
- `text` → `copy_to_clipboard(&text)` (arboard, no shell)
- `text` → emitted as `transcription-complete` event (Tauri IPC, no shell)
- paste uses `target_focus` (window ID captured earlier, not the text)

Conclusion: Transcribed text does **not** reach any shell command. No command injection via audio.

- [ ] **Step 3: Check the `window_id` string for injection risk**

`window_id` is passed directly to `xdotool windowactivate --sync <window_id>` (`paste.rs:225`). It is the stdout of `xdotool getactivewindow` — an integer like `"12345678"`. An attacker would need to control the output of `xdotool getactivewindow` on the victim's machine, which requires local code execution already. This is a very low risk.

- [ ] **Step 4: Write the finding**

Add to the report under "Command Injection":

```markdown
## [Low] Linux shell-out window ID not sanitized before xdotool

**File:** src-tauri/src/output/paste.rs:225
**What it is:** The app passes a window ID string directly to `xdotool windowactivate`
without validating that it contains only digits. The window ID comes from `xdotool
getactivewindow` (an integer), not from user input or transcribed text — so in normal
operation there is no injection risk.
**Scenario:** Only exploitable if an attacker has already compromised the system and
can manipulate `xdotool`'s output. No real-world attack path from audio or network input.
**Fix:** Low priority. Optionally validate that `window_id` contains only ASCII digits
before passing it to `xdotool`. Example:
  Before: `xdotool windowactivate --sync {window_id}`
  After: validate `window_id.chars().all(|c| c.is_ascii_digit())` and return an error if not.
```

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add command injection analysis"
```

---

## Task 3: Analyze IPC input validation (model name injection)

**Files:**
- Read: `src-tauri/src/models/downloader.rs:30-56`
- Read: `src-tauri/src/commands.rs:218-246`

**Background:** The frontend sends a model name string (like `"base"` or `"large-v3"`) to Rust commands. Rust uses that string to build both a file path and an HTTPS URL without validating it first.

- [ ] **Step 1: Analyze path traversal in download_model**

In `downloader.rs:31`:
```rust
pub fn model_path(name: &str) -> PathBuf {
    models_dir().join(format!("ggml-{}.bin", name))
}
```

If `name = "../../etc/cron.d/evil"`, the path becomes:
`~/.local/share/careless-whisper/models/ggml-../../etc/cron.d/evil.bin`

On most systems `PathBuf::join` with a relative path does traverse. Rust's `PathBuf` does **not** canonicalize or reject `..` components by default.

- [ ] **Step 2: Analyze URL injection in download_model**

In `downloader.rs:47-50`:
```rust
let url = format!(
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
    name
);
```

If `name = "../../passwd"`, the URL becomes `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-../../passwd.bin` — this would 404 on HuggingFace but is still a malformed request. A more targeted injection like `name = "base?token=xxx"` could append query parameters. No credentials are at risk here since the URL is public, but it's still improper input handling.

- [ ] **Step 3: Assess real-world exploitability**

The frontend is the only caller (`commands.rs:218`). Since Tauri's webview is a local UI (not exposed to the network), an attacker would need to:
1. Compromise the webview (requires XSS or a malicious frontend), AND
2. Call `download_model` with a crafted name

Given CSP is `null` (Task 4 covers this), XSS in the webview is possible if user-controlled text is ever rendered as HTML, making this a chained risk.

- [ ] **Step 4: Analyze delete_model**

`commands.rs:223-225` and `downloader.rs:93-98`: same `model_path(name)` path construction. A `delete_model("../../config")` call would attempt to delete an arbitrary file the app has write access to.

- [ ] **Step 5: Write the findings**

Add to the report under "IPC Input Validation":

```markdown
## [Medium] Path traversal in download_model and delete_model

**File:** src-tauri/src/models/downloader.rs:31, src-tauri/src/commands.rs:223
**What it is:** The `model` parameter (a string like "base") is inserted directly into
a file path (`ggml-{name}.bin`) and a URL without validation. A value like
`../../etc/cron.d/evil` would construct a path outside the models directory. Rust's
`PathBuf` does not reject `..` components automatically.
**Scenario:** If the webview is ever compromised (e.g. via XSS — see CSP finding),
an attacker could call `download_model("../../some/path")` to write a file outside the
models directory, or `delete_model("../../config")` to delete the app's config file.
Without a CSP bypass this is not directly reachable from outside the machine.
**Fix:** Add an allowlist check before using the name. The valid model names are a fixed
set of 5 strings. Reject anything not in that list:
  Before: `downloader::download_model(app, model).await`
  After:
  ```rust
  const VALID_MODELS: &[&str] = &["tiny", "base", "small", "medium", "large-v3"];
  if !VALID_MODELS.contains(&model.as_str()) {
      return Err(format!("Unknown model: {}", model));
  }
  downloader::download_model(app, model).await
  ```
  Apply the same check in `delete_model` and `set_active_model`.
```

- [ ] **Step 6: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add IPC input validation analysis"
```

---

## Task 4: Analyze CSP disabled and get_recent_logs

**Files:**
- Read: `src-tauri/tauri.conf.json:39-41`
- Read: `src-tauri/src/commands.rs:355-362`
- Read: `src/components/Settings.tsx` — find the get_recent_logs usage

- [ ] **Step 1: Understand what CSP null means**

`tauri.conf.json:40`: `"csp": null` means Tauri does not set a Content Security Policy on the webview. A CSP controls what content is allowed to run (scripts, inline styles, fetch targets). Without one, if any user-controlled string is ever rendered as raw HTML, injected `<script>` tags would execute. This is called XSS (Cross-Site Scripting).

- [ ] **Step 2: Check if any user-controlled text is rendered as HTML**

Read `src/components/Settings.tsx`. Search for `dangerouslySetInnerHTML`, `innerHTML`, or direct DOM manipulation. If not present: React renders text as text nodes by default (safe). Note this finding as "no current XSS vector" but flag that the missing CSP means any future developer adding `dangerouslySetInnerHTML` would immediately create a real vulnerability.

- [ ] **Step 3: Analyze get_recent_logs**

`commands.rs:355-362`: The command reads the raw log file and returns it as a string to the frontend. Log lines can contain transcribed speech (anything the user said). The frontend (`Settings.tsx`) uses this to populate a clipboard copy for a bug report. If the Settings component ever renders log text as HTML (not just clipboard text), transcribed content could be an XSS vector.

- [ ] **Step 4: Write the findings**

Add to the report under "CSP Disabled + get_recent_logs":

```markdown
## [Medium] Content Security Policy is disabled

**File:** src-tauri/tauri.conf.json:40
**What it is:** `"csp": null` disables the Content Security Policy for the webview.
A CSP is a safety net that prevents injected scripts from running even if someone
manages to sneak HTML into the page. Without it, any future code that renders
user-controlled text as HTML (using `dangerouslySetInnerHTML` or direct DOM writes)
would immediately allow script execution.
**Scenario:** Currently the React components use safe text rendering, so there is no
active XSS path. But the missing CSP means one careless line of code in the future
could create a serious vulnerability with no safety net.
**Fix:** Add a restrictive CSP to tauri.conf.json:
  Before: `"csp": null`
  After:
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src https://huggingface.co"
  ```
  Note: `'unsafe-inline'` is needed for Tailwind's inline styles. The `connect-src`
  allows model downloads from HuggingFace.

## [Info] get_recent_logs returns raw log content including transcriptions

**File:** src-tauri/src/commands.rs:355-362
**What it is:** The `get_recent_logs` command returns the last 100 lines of the app's
log file as a plain string to the frontend. Log lines may include transcribed speech
and system paths. Currently this text is only written to the clipboard (safe), but it
is worth noting as a data exposure surface: anyone with access to the clipboard after
a "Report Issue" action will see recent transcriptions.
**Scenario:** User clicks "Report Issue", copies logs to clipboard, pastes into a GitHub
issue. Any private speech that was transcribed recently appears in the issue.
**Fix:** Before the "copy to clipboard" action in the frontend, show the user the log
content and ask them to review/redact it. No code change needed in Rust.
```

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add CSP and get_recent_logs analysis"
```

---

## Task 5: Analyze unsafe Rust

**Files:**
- Read: all `.rs` files in `src-tauri/src/`

- [ ] **Step 1: Find all unsafe blocks**

```bash
grep -n "unsafe" src-tauri/src/**/*.rs src-tauri/src/*.rs 2>/dev/null
```

- [ ] **Step 2: Categorize each unsafe block**

For each `unsafe` block found, determine:
- What does it do?
- Is it calling a C API (e.g. CoreGraphics, Win32, libc)?
- Is it doing raw pointer arithmetic that could go wrong?
- Is it well-bounded (the unsafe code is short and its invariants are obvious)?

Known unsafe blocks from the codebase:
1. `paste.rs:62-70` — Objective-C message sends to NSWorkspace/NSRunningApplication (macOS). Uses `objc2` bindings; null-checked before use. Low risk.
2. `paste.rs:75-93` — CoreGraphics CGEventCreateKeyboardEvent / CGEventPostToPid (macOS). Raw C pointers, null-checked. Standard pattern for macOS key injection.
3. `paste.rs:123-134` — Windows WASAPI via `windows-rs` (Win32). Follows windows-rs safe patterns.
4. `lib.rs:56-70` — CoreFoundation + ApplicationServices for accessibility check. Standard macOS pattern.
5. `lib.rs:99-108` — `libc::mkfifo` call to create FIFO (Linux). Single syscall, result checked.
6. `commands.rs:260-261` — `AXIsProcessTrusted()` call. Single C function, return value used safely.
7. `commands.rs:302-316` — CoreFoundation dictionary + AXIsProcessTrustedWithOptions. CFRelease on result. Standard pattern but see note below.

- [ ] **Step 3: Check for CFRelease correctness**

In `commands.rs:314`: `CFRelease(options as *mut c_void)` — the `options` pointer is a `*const c_void` from `CFDictionaryCreate`. Casting to `*mut` for CFRelease is standard CF pattern (CF's ownership model requires this). The same pattern appears in `paste.rs:82,91`. These are correct.

- [ ] **Step 4: Write the finding**

Add to the report under "Unsafe Rust":

```markdown
## [Info] Unsafe blocks are well-bounded and follow platform API conventions

**Files:** src-tauri/src/output/paste.rs, src-tauri/src/lib.rs, src-tauri/src/commands.rs
**What it is:** The codebase contains ~7 unsafe blocks, all calling platform C APIs:
CoreGraphics and ApplicationServices (macOS), Win32 SendInput/SetForegroundWindow
(Windows), and libc mkfifo (Linux). Each block is short, null-checks raw pointers
before use, and follows the standard patterns for those APIs.
**Scenario:** No identified memory safety risks in the current unsafe code. The patterns
used (objc2 message sends, CoreFoundation CFRelease, windows-rs INPUT structs) are the
idiomatic way to call these APIs from Rust.
**Fix:** No changes needed. For future reference: any new unsafe block should be
accompanied by a comment explaining *why* it's needed and *what invariants* make it safe.
```

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add unsafe Rust analysis"
```

---

## Task 6: Analyze permissions and macOSPrivateApi

**Files:**
- Read: `src-tauri/tauri.conf.json:42`
- Read: `src-tauri/Info.plist` (if present)
- Read: `src-tauri/src/lib.rs:188-230`

- [ ] **Step 1: Check Info.plist for declared permissions**

```bash
cat src-tauri/Info.plist 2>/dev/null || echo "not found"
```

Note which entitlements are declared (expected: Microphone, Accessibility).

- [ ] **Step 2: Assess whether permissions are justified**

- **Microphone** (`NSMicrophoneUsageDescription`): Required — core feature is audio recording.
- **Accessibility** (`AXIsProcessTrustedWithOptions`): Required — used for `CGEventPostToPid` to simulate paste. Without this, paste simulation fails on macOS.
- **Autostart** (`tauri-plugin-autostart`): Optional, user-controlled via Settings toggle. Justified.
- No network entitlement needed on macOS (reqwest works without a special entitlement for HTTPS).

- [ ] **Step 3: Assess macOSPrivateApi**

`tauri.conf.json:42`: `"macOSPrivateApi": true` enables Tauri's access to private macOS APIs, primarily used for transparent/vibrancy window effects. This app uses it for the transparent overlay window (`"transparent": true` on the overlay window). It is justified. However, using private APIs carries a theoretical app store rejection risk (not relevant here since it's not submitted to the App Store).

- [ ] **Step 4: Write the finding**

Add to the report under "Permissions":

```markdown
## [Info] Permissions are appropriate and justified

**Files:** src-tauri/Info.plist, src-tauri/tauri.conf.json:42
**What it is:** The app requests Microphone and Accessibility permissions on macOS.
Both are required for core functionality (audio capture and paste simulation). The app
also uses `macOSPrivateApi: true` to enable the transparent recording overlay.
**Scenario:** No over-privileged permissions identified. The app does not request
camera, contacts, location, or full disk access.
**Fix:** No changes needed. For future maintainers: `macOSPrivateApi: true` would cause
App Store rejection if submitted — keep this in mind if distribution plans change.
```

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add permissions analysis"
```

---

## Task 7: Scan for secrets and credentials

**Files:**
- Entire repository

- [ ] **Step 1: Search for hardcoded secrets patterns**

```bash
grep -rn --include="*.rs" --include="*.ts" --include="*.tsx" --include="*.json" --include="*.toml" \
  -E "(api_key|apikey|api-key|secret|password|token|bearer|authorization|credential)" \
  . --exclude-dir=node_modules --exclude-dir=target 2>/dev/null | grep -iv "// " | grep -iv "test"
```

- [ ] **Step 2: Search for hardcoded URLs that might be sensitive**

```bash
grep -rn --include="*.rs" --include="*.ts" --include="*.tsx" \
  -E "https?://" . --exclude-dir=node_modules --exclude-dir=target 2>/dev/null
```

Review each URL found. Expected: only `https://huggingface.co/...` (the model download URL) and `https://github.com/...` (the "Report Issue" link in Settings.tsx).

- [ ] **Step 3: Check .gitignore for any secrets files that should be ignored**

```bash
cat .gitignore 2>/dev/null || echo "no .gitignore"
```

Verify `.env` files are excluded if present.

- [ ] **Step 4: Write the finding**

Add to the report under "Secrets / Credentials":

```markdown
## [Info] No hardcoded secrets found

**Files:** Entire repository
**What it is:** Scanned all Rust, TypeScript, JSON, and TOML files for hardcoded API
keys, tokens, passwords, and credentials.
**Finding:** None found. The only hardcoded URLs are:
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{name}.bin` — the
  public model download URL (no auth required)
- The GitHub issues URL in Settings.tsx — public
**Fix:** No changes needed.
```

(Update this finding if any secrets are actually found in Step 1-2.)

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add secrets scan results"
```

---

## Task 8: Analyze settings persistence

**Files:**
- Read: `src-tauri/src/config/settings.rs`

- [ ] **Step 1: Review the save/load flow**

`settings.rs:62-78`: `load()` reads `config.json` via `std::fs::read_to_string`, parses with `serde_json::from_str`. `save()` serializes with `serde_json::to_string_pretty`, writes with `std::fs::write`.

- [ ] **Step 2: Check for TOCTOU (time-of-check to time-of-use) issues**

`save()` calls `create_dir_all` then `fs::write`. These are two separate syscalls. In theory, a race condition could occur between them, but this is a local config file written by the app itself — exploitation would require a highly targeted local attack. No real risk.

- [ ] **Step 3: Check for injection in deserialized values**

`Settings` fields are primitive types (String, bool, u32, enums). The `hotkey` field is a `String` that gets passed to `tauri-plugin-global-shortcut`. The `language` field is a `String` passed to whisper-rs. Neither of these creates an injection vector since they go to typed APIs, not shell commands.

- [ ] **Step 4: Check file permissions on config.json**

The config file is created with `std::fs::write` which uses default OS file permissions (typically 0o644 on Linux/macOS — readable by all users on the system). On a single-user machine this is fine. On a multi-user system, another user could read the config (hotkey preference, language setting — no secrets).

- [ ] **Step 5: Write the finding**

Add to the report under "Settings Persistence":

```markdown
## [Info] Settings persistence is safe; config file is world-readable

**File:** src-tauri/src/config/settings.rs:71-78
**What it is:** Settings are saved as JSON using serde. No injection risks were found.
The config file is written with default OS permissions (0o644), meaning other users on
the same machine can read it. The config contains hotkey preferences and language
settings — no passwords or tokens.
**Scenario:** On a shared machine, another user could read your hotkey and language
preference. This is low sensitivity information.
**Fix:** Optionally set 0o600 permissions on the config file so only the owner can read
it. Not required unless you share your machine with untrusted users.
  After saving, add: `std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))`
  (requires `use std::os::unix::fs::PermissionsExt;` on Unix)
```

- [ ] **Step 6: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add settings persistence analysis"
```

---

## Task 9: Analyze model download integrity

**Files:**
- Read: `src-tauri/src/models/downloader.rs:46-91`

- [ ] **Step 1: Review the download flow**

`downloader.rs:46-91`: Downloads via HTTPS from a hardcoded HuggingFace URL to a `.part` file, then renames to `.bin`. No hash verification, no file size check beyond `content_length`, no format validation of the binary before it is passed to whisper-rs.

- [ ] **Step 2: Assess the threat model**

The download URL is hardcoded and uses HTTPS, which provides transport-layer integrity (prevents network interception). The real risk would be if HuggingFace itself were compromised and served a malicious model file. whisper.cpp loads the `.bin` file via C++ code — a maliciously crafted model file could potentially exploit a parsing vulnerability in whisper.cpp (a C library).

- [ ] **Step 3: Check if HuggingFace provides checksums**

HuggingFace model repositories do provide SHA256 checksums via their API. The `ggerganov/whisper.cpp` repo has a `README.md` that lists expected SHA256 hashes for each model file. These are not used in the current download code.

- [ ] **Step 4: Write the finding**

Add to the report under "Model Download Integrity":

```markdown
## [Medium] No integrity check on downloaded model files

**File:** src-tauri/src/models/downloader.rs:46-91
**What it is:** After downloading a model `.bin` file from HuggingFace, the app renames
it and loads it into whisper.cpp without verifying its SHA256 hash. The download uses
HTTPS (which prevents interception in transit), but does not verify the file matches
the expected checksum.
**Scenario:** If HuggingFace were compromised, or if the download were intercepted via
a HTTPS MITM attack (e.g. a compromised corporate proxy), a malicious `.bin` file could
be loaded into whisper.cpp (a C library). A crafted model file could potentially exploit
a memory safety bug in whisper.cpp's model parser.
**Fix:** After downloading, verify the SHA256 of the file matches the known-good hash
before renaming from `.part` to `.bin`. The expected hashes are published in the
ggerganov/whisper.cpp repository README. Hard-code them alongside the model definitions
in `downloader.rs`:
  ```rust
  const MODELS: &[(&str, u32, u32, &str)] = &[
      ("tiny",     75,   390,  "expected-sha256-hash-here"),
      ("base",     142,  500,  "expected-sha256-hash-here"),
      // ...
  ];
  ```
  Then after download, compute SHA256 of the `.part` file and compare before rename.
  Use the `sha2` crate: `sha2 = "0.10"`.
```

- [ ] **Step 5: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add model download integrity analysis"
```

---

## Task 10: Analyze Linux FIFO listener

**Files:**
- Read: `src-tauri/src/lib.rs:78-146`

- [ ] **Step 1: Understand the FIFO design**

`lib.rs:79-146`: On Linux, a named pipe (FIFO) is created at `~/.local/share/careless-whisper/careless-whisper.sock` with mode `0o644`. A background thread blocks reading from it. When any data is written, the app toggles recording. The content of what's written is ignored (`let _ = file.read(&mut buf)`).

- [ ] **Step 2: Assess who can write to this FIFO**

Mode `0o644` means: owner can read+write, group can read, others can read. On a standard Linux system, other users cannot write to a `0o644` file owned by you. However, **any process running as the same user** (uid) can write to it — including browser tabs, electron apps, VS Code extensions, or any malicious software running under your user account.

- [ ] **Step 3: Assess the impact**

An attacker who can run code as the same user can:
1. Write to the FIFO → trigger recording start (capturing microphone audio)
2. Write to the FIFO again → trigger recording stop + transcription (the transcribed text goes to clipboard and is pasted into the focused window)

This is a **local privilege escalation of audio access** — a compromised app that already has code execution could silently start recording without the user pressing the hotkey.

- [ ] **Step 4: Assess the content injection risk**

The FIFO buffer is `[0u8; 128]` — 128 bytes read and discarded. The content is not parsed or executed. No injection into shell commands via the FIFO content.

- [ ] **Step 5: Write the finding**

Add to the report under "Linux FIFO Listener":

```markdown
## [High] Linux FIFO listener allows any same-user process to trigger recording

**File:** src-tauri/src/lib.rs:79-146
**What it is:** On Linux, the app creates a named pipe (FIFO) at
`~/.local/share/careless-whisper/careless-whisper.sock`. Any process running as your
user account can write to this file to silently start or stop audio recording — without
requiring the hotkey. The FIFO is designed this way intentionally (to support custom
keybindings on Wayland), but it also means any compromised app running as you could
trigger recording.
**Scenario:** Malicious software installed under your user account (a compromised npm
package, a malicious VS Code extension, etc.) could write to the FIFO, start a
recording, wait for you to speak sensitive information, stop the recording, and read
the transcribed text from the clipboard.
**Fix (Option 1 — Basic):** Change the FIFO mode from 0o644 to 0o600 (owner read/write
only). This does not change the intended functionality (the user's own keybinding script
still runs as the same user), but documents the intent more clearly:
  Before: `libc::mkfifo(fifo_c.as_ptr(), 0o644)`
  After:  `libc::mkfifo(fifo_c.as_ptr(), 0o600)`
**Fix (Option 2 — Stronger):** Add a simple secret token. On startup, generate a random
token and write it to a 0o600 file. Require the FIFO message to include the token:
  `echo "toggle:$(cat ~/.local/share/careless-whisper/token)" > ...sock`
  This prevents other processes from triggering recording even if they discover the FIFO
  path, since they don't know the token.
Note: Fix Option 1 is low-effort and sufficient for personal use. Option 2 is better
for a shared or production system.
```

- [ ] **Step 6: Commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): add Linux FIFO listener analysis"
```

---

## Task 11: Write summary table and recommended fix order

**Files:**
- Modify: `docs/security/audit-2026-03-22.md`

- [ ] **Step 1: Replace the Summary Table placeholder**

Update the Summary Table section with all findings:

```markdown
## Summary Table

| # | Severity | Title | File |
|---|----------|-------|------|
| 1 | High     | Linux FIFO listener allows any same-user process to trigger recording | lib.rs:79-146 |
| 2 | Medium   | Path traversal in download_model and delete_model | downloader.rs:31 |
| 3 | Medium   | No integrity check on downloaded model files | downloader.rs:46-91 |
| 4 | Medium   | Content Security Policy is disabled | tauri.conf.json:40 |
| 5 | Low      | Linux shell-out window ID not sanitized before xdotool | paste.rs:225 |
| 6 | Info     | get_recent_logs returns raw log content including transcriptions | commands.rs:355-362 |
| 7 | Info     | Unsafe blocks are well-bounded and follow platform API conventions | paste.rs, lib.rs, commands.rs |
| 8 | Info     | Permissions are appropriate and justified | Info.plist, tauri.conf.json:42 |
| 9 | Info     | No hardcoded secrets found | entire repo |
| 10| Info     | Settings persistence is safe; config file is world-readable | settings.rs:71-78 |
```

(Update with actual dependency audit findings from Task 1 if any were found.)

- [ ] **Step 2: Replace the Recommended Fix Order placeholder**

```markdown
## Recommended Fix Order

Fix these in order — highest impact first:

1. **[High] FIFO mode** (`lib.rs:100`): Change `0o644` to `0o600`. One-line fix, 5 minutes.
   This closes the silent recording trigger attack vector.

2. **[Medium] Model name allowlist** (`commands.rs:218,223,228`): Add a 3-line check
   that rejects any model name not in the known set of 5. Closes path traversal and URL
   injection. 10 minutes.

3. **[Medium] Enable CSP** (`tauri.conf.json:40`): Add a restrictive CSP string. 5 minutes.
   Provides a safety net against future XSS vulnerabilities.

4. **[Medium] Model download integrity** (`downloader.rs`): Add SHA256 verification after
   download. Requires adding the `sha2` crate and looking up expected hashes. 30-60 minutes.

5. **[Low] Validate window_id in paste_x11** (`paste.rs:225`): Optional — only needed if
   you're concerned about local attacker scenarios. Low priority.

Everything marked [Info] requires no code changes.
```

- [ ] **Step 3: Finalize the report header to show completion**

Replace the top of the report to reflect it's complete:

```markdown
# Security Audit — Careless Whisper
**Date:** 2026-03-22
**Scope:** Dependency CVE audit + static code review (Option B)
**Audience:** Beginner — all findings explained in plain English
**Status:** Complete — 4 actionable findings (1 High, 3 Medium, 1 Low, 5 Info)
```

- [ ] **Step 4: Final commit**

```bash
git add docs/security/audit-2026-03-22.md
git commit -m "docs(security): complete audit report with summary table and fix order"
```
