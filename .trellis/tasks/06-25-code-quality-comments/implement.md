# Code Quality and Architecture Comments Implementation Plan

## Preconditions

- Task status must be `in_progress` before editing production/spec files.
- Before implementation, read:
  - `.trellis/spec/frontend/index.md`
  - `.trellis/spec/frontend/quality-guidelines.md`
  - `.trellis/spec/frontend/type-safety.md`
  - `.trellis/spec/backend/comment-guidelines.md`
  - `.trellis/spec/guides/cross-layer-thinking-guide.md`

## Ordered Checklist

### 1. Frontend Type Safety Spec

- [x] Replace placeholder content in `.trellis/spec/frontend/type-safety.md`.
- [x] Document `src/types/*.ts` as Tauri IPC contract mirrors.
- [x] Document optional versus nullable field rules.
- [x] Document when exported TS interfaces and fields require comments.
- [x] Document forbidden patterns: `any`, local payload casts, duplicated contract types.

Review gate:

- [ ] Spec remains concise and project-specific.
- [ ] Spec does not invent tooling or future-only requirements.

### 2. Frontend Contract Types

- [x] Audit and update `src/types/sync.ts`.
- [x] Audit and update `src/types/task.ts`.
- [x] Audit and update `src/types/course.ts`.
- [x] Audit and update `src/types/exam.ts`.
- [x] Audit and update `src/types/pomodoro.ts`.
- [x] Audit and update `src/types/schedule.ts`.
- [x] Audit and update `src/types/course-import.ts` if its contract has ambiguous fields.

Implementation notes:

- Add type-level comments for backend alignment.
- Add field comments only for ambiguous fields.
- Add `sync_id` and `deleted_at` to frontend return models when backend returns those fields.
- Do not change command input shapes unless TypeScript currently conflicts with the Rust command payload.

Review gate:

- [ ] No UI rendering changes.
- [ ] No broad rename or abstraction.
- [ ] Optional fields are not changed to required unless backend command requires them.

### 3. Backend High-Risk Comment Gaps

- [x] Audit `src-tauri/src/schedule.rs` response structs and constants.
- [x] Add minimal comments to schedule/calendar response fields that cross the Rust/TypeScript boundary.
- [x] Audit `src-tauri/src/db/models.rs` for remaining high-risk field ambiguity.
- [x] Audit `src-tauri/src/sync/exporter.rs`, `src-tauri/src/sync/webdav.rs`, and `src-tauri/src/commands/sync.rs` for obvious missing protocol comments only.

Review gate:

- [ ] Comments follow existing Chinese comment language.
- [ ] No behavior changes unless a low-risk contract mismatch is impossible to avoid.
- [ ] No CRUD wrapper noise comments.

### 4. Verification

- [x] Run `npm run lint`.
- [x] Run `npm run build`.
- [x] If Rust files changed, run `cargo fmt --check` in `src-tauri`.
- [x] If Rust non-comment behavior changed, run relevant `cargo test` scope; otherwise document that Rust behavior was unchanged.

### 5. Final Review

- [x] Confirm `git diff` only includes task docs, specs, comments, and type contract alignment.
- [x] Confirm no generated assets or dependency files changed unexpectedly.
- [x] Summarize changes and validation results to user.
- [x] Do not commit unless explicitly requested by user.

## Rollback Points

- After Step 1: revert spec update only.
- After Step 2: revert frontend type files only.
- After Step 3: revert Rust comments only.
- After verification failure: prefer targeted fix; if failure is unrelated, record it and do not mask with broad changes.
