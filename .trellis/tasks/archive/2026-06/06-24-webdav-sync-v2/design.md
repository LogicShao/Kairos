# WebDAV sync v2 design

## Context

The current sync format exports local rows directly and merges by SQLite `id`. That works for a single device but fails for multi-device or delete/update propagation because SQLite ids are not globally stable. WebDAV should be treated as remote file storage, while Kairos owns the sync protocol.

## Protocol

Kairos will keep one remote file:

```text
kairos-sync.json
```

The v2 JSON envelope:

```json
{
  "schema_version": 2,
  "dataset_id": "uuid",
  "device_id": "uuid",
  "exported_at": "2026-06-24T10:00:00Z",
  "tasks": [],
  "courses": [],
  "exams": [],
  "pomodoro_sessions": []
}
```

Each synced entity includes:

- `sync_id`: stable UUID shared across devices.
- `created_at`: original creation timestamp.
- `updated_at`: last mutation timestamp.
- `deleted_at`: `null` for active entities, ISO timestamp for tombstones.

Local SQLite `id` remains local-only and is never used as the cross-device identity.

## Database Changes

Add a migration after v3:

- `sync_config.remote_etag TEXT`
- `sync_config.device_id TEXT`
- `sync_config.dataset_id TEXT`
- `tasks.sync_id TEXT`
- `tasks.deleted_at TEXT`
- `courses.sync_id TEXT`
- `courses.deleted_at TEXT`
- `exams.sync_id TEXT`
- `exams.deleted_at TEXT`
- `pomodoro_sessions.sync_id TEXT`
- `pomodoro_sessions.deleted_at TEXT`

Backfill:

- Existing rows receive newly generated UUID-like ids.
- `device_id` and `dataset_id` are generated if absent.
- Unique indexes are added on non-null `sync_id` per entity table.

Soft-delete:

- Task/course/exam deletes set `deleted_at` and `updated_at`.
- Normal list/query APIs filter out `deleted_at IS NOT NULL`.
- Pomodoro sessions may remain append-only; tombstone support is still added for protocol consistency.

## Merge Semantics

Merge key:

- `sync_id` when present.
- v1 fallback: local `id` only for legacy import.

Last-writer-wins per entity:

- If remote entity is missing locally, insert it unless it is an old tombstone that only exists remotely.
- If both exist, compare the newest effective timestamp:
  - `deleted_at` when present, otherwise `updated_at`.
- Newer version wins.
- If timestamps are equal, keep local.

Deletion:

- A tombstone with a newer timestamp marks local entity deleted.
- Tombstones are exported for a retention window or indefinitely for MVP.

## WebDAV HTTP Semantics

Download response should include:

- Parsed sync data.
- Remote ETag, if present.

Upload:

- If an ETag is known, send `If-Match: <etag>`.
- If no remote file exists, create normally.
- If server returns `412 Precondition Failed`, treat as a remote-change conflict.

Retry:

1. Download remote.
2. Import/merge into local.
3. Export merged snapshot.
4. Upload again with the new ETag.
5. If retry also conflicts, return a recoverable sync error.

Do not use WebDAV `LOCK`; it is not reliably supported across common providers.

## Compatibility

V1 remote file:

- Existing `SyncData` without `schema_version` is accepted as v1.
- V1 rows are normalized into v2 entities by deriving or assigning `sync_id`.
- After successful sync, upload v2 format.

Existing local database:

- Migration/backfill makes existing rows syncable without data loss.
- UI must continue using numeric `id` for local operations.

## Frontend Contract

`sync_now` continues returning `SyncResult`, but may extend stats with conflict/retry details if needed. Existing frontend can keep rendering merged counts and uploaded/downloaded status.

## Tradeoffs

- Snapshot v2 is simpler than operation logs and adequate for current data volume.
- Tombstones keep deletes safe but require query filters and eventual cleanup policy later.
- LWW avoids complex UI but can lose one side of simultaneous edits. This is acceptable for MVP.
- ETag conditional upload protects against overwriting remote changes but depends on provider support. If ETag is absent, sync falls back to timestamp-based behavior.

## Rollback

- Database migrations are forward-only; rollback is via backup/restore or keeping tombstoned data dormant.
- Remote v2 file can be replaced by the last v1 file only manually; the app should be able to read v1 but not necessarily write it after migration.
- If ETag upload causes provider-specific issues, conditional upload can be disabled while retaining `sync_id` and tombstone semantics.
