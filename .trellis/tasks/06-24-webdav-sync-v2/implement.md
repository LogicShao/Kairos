# WebDAV sync v2 implementation plan

## Preconditions

- Preserve existing uncommitted changes:
  - `src/components/sync/SyncSettings.tsx`
  - `src-tauri/src/commands/sync.rs`
- Do not commit without explicit user confirmation.
- Keep edits scoped to sync/database code unless frontend result display needs a small update.

## Steps

1. Add sync metadata primitives.
   - Create helper for generating stable ids.
   - Prefer an existing dependency if available; otherwise add a small local random/chrono-based helper only if defensible.
   - Add typed v2 sync envelope/entity structs.

2. Add database migration.
   - Add `sync_id` and `deleted_at` to entity tables.
   - Add `remote_etag`, `device_id`, `dataset_id` to `sync_config`.
   - Backfill missing values safely.
   - Add migration tests for idempotency and backfill.

3. Update Rust models and CRUD queries.
   - Include new fields in `Task`, `Course`, `Exam`, `PomodoroSession`.
   - Filter active list queries with `deleted_at IS NULL`.
   - Change deletes for task/course/exam to soft delete.
   - Keep UI-facing numeric ids unchanged.

4. Rework sync import/export.
   - Export v2 snapshots with `schema_version = 2`.
   - Parse both v1 and v2 remote files.
   - Merge by `sync_id`.
   - Implement tombstone propagation.
   - Add unit tests for course edit and delete propagation.

5. Extend WebDAV client.
   - Download returns body plus ETag.
   - Upload accepts optional ETag and sends `If-Match`.
   - Surface 412 as a typed conflict error, not a generic string where possible.
   - Add pure/unit tests for ETag handling where feasible without real network.

6. Update `sync_now`.
   - Download remote and ETag.
   - Merge.
   - Upload with conditional request.
   - On 412, retry download/merge/upload once.
   - Persist latest remote ETag and sync timestamp after successful upload.

7. Frontend compatibility pass.
   - Update TypeScript sync result types only if backend response adds fields.
   - Keep existing UI behavior stable.

8. Verification.
   - `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
   - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `npx tsc --noEmit`
   - `npm run lint`
   - `npm run build`
   - `git diff --check`

## Review Gates

- After migration step: run migration-focused tests.
- After merge step: run sync exporter tests.
- After WebDAV step: run sync command/client tests.
- Before final response: run the full verification set.

## Rollback Points

- Before migration: no persistent schema changes.
- After migration but before CRUD soft-delete: schema can remain unused if implementation is paused.
- After v2 exporter: local app can still read v1 if parser is kept compatible.

## Known Risks

- ETag support varies by provider; implementation must handle missing ETags.
- Soft deletes require every normal query path to filter tombstones.
- Existing exams linked by local `course_id` may need careful handling when remote courses are inserted with a new local id.
