# AI Morning Brief Contract

> Design decisions and runtime contracts for the AI daily-brief feature
> (`src-tauri/src/ai/*`, `commands/ai.rs`, `db/ai.rs`). Codified 2026-07-31 from
> the 4-dimension parallel design + adversarial review of task
> `07-31-ai-morning-brief`. Keep this as the single source of truth so future
> AI sessions do not reintroduce conflicting contracts.

---

## 1. base_url never contains `/v1`

- `ai_config.base_url` default is `https://api.deepseek.com` — **no** `/v1`.
- The code appends `/v1/chat/completions` once: `format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))`.
- Migration seed and `update_ai_config` must store host-without-`/v1`. Storing `/v1`
  in the DB produces `https://api.deepseek.com/v1/v1/chat/completions` → every request 404s.

## 2. API key encryption (do not reuse `lzu/crypto.rs`)

`lzu/crypto.rs` is AES-CBC-128 with `IV = Key` and zero padding — a weak config
declared "not to be reused" in its own file header. The AI feature has its own module:

- `src-tauri/src/ai/crypto.rs`: AES-128-CBC + **random 16-byte IV per encryption** + PKCS7.
- Key: 16 random bytes written once to `app_data_dir/.ai_encryption_key`
  (`ensure_key_file`; entropy from `Uuid::new_v4().into_bytes()`, zero new deps; Unix chmod 600).
- Stored format: `hex(iv) || ":" || hex(cipher)`.
- `update_ai_config` `api_key: Option<String>` semantics:
  `Some(non-empty)` = encrypt & write new key; `Some("")` = clear; `None` = keep existing cipher.
- Never return plaintext/masked key over IPC: `AiConfigView.api_key_configured` bool only.
- Threat model: protects against **remote/WebDAV/static single-file leaks**, NOT local
  root (key file and `kairos.db` share `app_data_dir`).

## 3. Sync whitelist excludes AI tables

`sync/exporter.rs::SyncData` explicitly lists only
tasks/courses/exams/pomodoro_sessions/term_phases. `ai_config` and `ai_morning_brief`
are device-local and must **never** be added to `SyncData`. Regression test:
`sync::exporter::tests::test_export_excludes_ai_tables`.

## 4. Command layer: sync commands + blocking reqwest + narrow lock

- All AI commands are **synchronous** `#[tauri::command]` using `reqwest::blocking`
  (Tauri command thread pool absorbs blocking; matches `sync/webdav.rs` precedent).
- **Narrow lock discipline**: snapshot briefing data + read config inside the DB lock,
  **drop all locks before the HTTP call**, then re-lock for a double-check upsert.
  Never hold `Mutex<Connection>` across a network call (pomodoro tick worker grabs it every 1s).
- One module `commands/ai.rs` with four commands:
  `get_ai_config`, `update_ai_config(db, engine, app_handle, req)`,
  `get_ai_morning_brief`, `generate_ai_morning_brief(db, engine, app_handle, force)`.
- Frontend passes `{ req: ... }` / `{ force: ... }`; `State` and `AppHandle` params are auto-injected.

## 5. Per-day cache + in-flight guard (cost guardrail)

- `ai_morning_brief.date UNIQUE` (`+08:00` China date). `force=false` returns cache, zero cost.
- Module-level `OnceLock<Arc<AtomicBool>>` in-flight guard (in `ai/morning_brief.rs`)
  prevents manual + 7:00 concurrent double billing. The second caller re-checks cache,
  else returns "generating" error.
- `max_tokens` is hardcoded `400` (`prompt.rs::MAX_TOKENS`) — never promoted to config;
  the frontend cannot raise it.
- 7:00 scheduler (`ai/scheduler.rs`) uses a single cancel token (pomodoro_scheduler
  pattern), only runs when `enabled && api_key_encrypted non-empty`, and reschedules on
  config change.

## 6. Degraded output is structurally identical Markdown

- AI and rule engines produce the **same fixed 5-section skeleton** with different footers:
  - AI: `*AI生成，请核实*`
  - Rule: `*本地生成*`
- `source` values `"ai" | "rule"` are identical across DB `CHECK`, serde
  `rename_all="lowercase"`, and the frontend `BriefSource` union.
- `prompt::validate_ai_output` checks all 5 section titles + AI footer; on failure the
  orchestrator degrades to the rule engine — **no second API call**.
- User message trims `id` fields to cut tokens and is capped by
  `MAX_CONTEXT_CHARS=4000` (field trimming, never mid-JSON truncation).

## 7. Single model constant & cost budget

- `prompt.rs::MODEL_DEFAULT = "deepseek-chat"` is the one default; the settings page can
  override `ai_config.model`. Prices drift — re-verify the budget against the provider's
  current price table when the default model or pricing changes.
- Budget target: < $0.0005/call (system ~600 + user ~450 in + 400 out). Kept under
  budget by fixed `max_tokens=400`, trimmed JSON, per-day caching, and in-flight guard.

## 8. Frontend rendering: markdown subset, zero dependency

- `src/lib/markdown-subset.tsx`: ~70-line React-node renderer (`#`/`##`, `**bold**`,
  `*italic*`, `- ` lists, `---` rule, blank-line paragraphs). No `react-markdown`
  dependency; React nodes (no `dangerouslySetInnerHTML`) are XSS-safe. Unknown lines
  fall back to `<p>`.
- `AiBriefCard` hides entirely when AI is disabled/not configured (feature is opt-in).
