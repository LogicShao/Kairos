# WebDAV sync v2

## Goal

Rework Kairos WebDAV sync from an id-based append/merge snapshot into a reliable single-file snapshot protocol that can propagate updates and deletes without resurrecting stale remote data or duplicating the same course under different local SQLite ids.

## Requirements

- Keep WebDAV as the only remote sync backend and continue storing one remote JSON file.
- Treat local SQLite `id` values as local-only identifiers. Synced entities must use stable `sync_id` values to identify the same task, course, exam, or pomodoro session across devices.
- Add deletion propagation through tombstones, so deleting a course/task/exam locally removes it from other devices after sync.
- Preserve compatibility with existing local databases by backfilling `sync_id` for existing rows.
- Preserve compatibility with existing v1 remote `kairos-sync.json` by importing it once and assigning missing `sync_id` values.
- Use WebDAV ETag and conditional upload to prevent silently overwriting a newer remote snapshot.
- Retry once after a conditional upload conflict by re-downloading, merging, and uploading the merged snapshot.
- Keep the frontend API shape stable enough that `sync_now` still returns a serializable `SyncResult`.
- Avoid relying on WebDAV `LOCK`, collection sync extensions, or server-specific behavior.

## Acceptance Criteria

- [ ] Editing a local course time and syncing does not create a duplicate course when the remote still contains the older version.
- [ ] Deleting a local course and syncing propagates the deletion to a second device after that device syncs.
- [ ] Two devices with different local SQLite ids for the same synced course merge by `sync_id`, not by `id`.
- [ ] Upload uses conditional request semantics when a remote ETag is known.
- [ ] If upload fails because the remote changed, sync re-downloads remote data, merges again, and retries once.
- [ ] Existing local rows receive `sync_id` values through migration or startup-safe backfill.
- [ ] Existing v1 remote data without schema version or `sync_id` can still be imported.
- [ ] Active UI lists and calendars do not show tombstoned rows.
- [ ] Rust tests cover update propagation, delete propagation, v1 import compatibility, ETag conflict retry logic, and migration idempotency.

## Definition of Done

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npx tsc --noEmit`
- `npm run lint`
- `npm run build`
- Sync behavior documentation or spec notes updated if the protocol contract changes.

## Out of Scope

- Full operation-log sync.
- Manual conflict resolution UI.
- Multi-file WebDAV collections or RFC 6578 collection sync.
- Server-side locking as the primary concurrency mechanism.
- End-to-end tests against a real WebDAV provider.

## Technical Notes

- Current backend sync files:
  - `src-tauri/src/commands/sync.rs`
  - `src-tauri/src/sync/exporter.rs`
  - `src-tauri/src/sync/webdav.rs`
  - `src-tauri/src/db/sync.rs`
  - `src-tauri/src/db/migrations.rs`
  - `src-tauri/src/db/models.rs`
- Current frontend sync page:
  - `src/components/sync/SyncSettings.tsx`
- Existing issue: current sync imports remote data before uploading local data and identifies entities by SQLite `id`, which allows stale remote records to be reintroduced.
- Preferred protocol: single-file JSON snapshot v2 with `schema_version`, `dataset_id`, `device_id`, `exported_at`, entity arrays, stable `sync_id`, and tombstone fields.
