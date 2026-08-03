# Code Quality and Architecture Comments Design

## Scope Boundary

This task uses the high-risk minimal scope selected by the user:

- Frontend type contracts under `src/types/*.ts`
- Frontend type safety spec at `.trellis/spec/frontend/type-safety.md`
- Obvious backend comment gaps in `src-tauri/src/db/models.rs`, `src-tauri/src/sync/*`, and `src-tauri/src/schedule.rs`

Out of scope:

- Full repository comment coverage
- Behavior changes to sync, schedule, database, or UI flows
- Database migrations
- Large refactors or new abstractions
- New lint/doc-generation dependencies

## Comment Strategy

Comments must stabilize cross-session AI interpretation. They should explain:

- Field meaning when the name is ambiguous
- Date/time format and timezone assumptions
- Sync identity versus local SQLite identity
- Null/empty-string semantics
- Cross-layer ownership and backend alignment
- Invariants or compatibility constraints

Comments must not explain:

- Local implementation steps already obvious from code
- Standard CRUD wrappers
- Pure presentation props with obvious names
- Repeated facts already covered by a nearby type-level comment

## Frontend Type Contract Strategy

`src/types/*.ts` are treated as Tauri IPC contract mirrors. They should document the shape that the frontend receives from Rust, not invent a separate domain model.

For each high-risk exported interface:

- Add a short type-level comment naming the backend owner file or command.
- Add field-level comments only for ambiguous fields.
- Prefer literal union types for known backend enum-like strings.
- Keep optional fields optional only when the frontend is allowed to omit them in command input.
- Return types should include backend fields if commands return them, even if the current UI does not render those fields.

### Sync Fields In Frontend Models

Backend v2 models include `sync_id` and `deleted_at` for tasks, courses, exams, and pomodoro sessions. Frontend list commands currently filter out soft-deleted rows but still return full model structs for active rows.

Design decision:

- Add `sync_id` and `deleted_at` to frontend entity return types if the corresponding Rust model includes them and Tauri commands return that model.
- Document that `sync_id` is a cross-device merge key and `deleted_at` is normally `null` in active UI queries.
- Do not render these fields or change UI behavior in this task.

Rationale:

- Omitting fields from TypeScript contracts makes later AI sessions think the fields do not exist.
- Adding them to types is a compile-time contract alignment, not a behavior change.

## Backend Comment Strategy

Existing backend sync comments are mostly adequate. Implementation should only add comments where a later AI would likely misinterpret:

- Schedule response structs in `src-tauri/src/schedule.rs`
- Constants whose meaning is not obvious (`EXAM_FALLBACK_COLOR`, task colors) if they influence cross-layer UI semantics
- Any missing field-level comments in DB models where sync or schedule semantics are non-obvious

Do not add comments to ordinary CRUD functions or simple command wrappers.

## Spec Strategy

Update `.trellis/spec/frontend/type-safety.md` from placeholder to project-specific guidance. It should include:

- Type organization under `src/types/`
- Tauri IPC contract mirroring rule
- Field comment trigger rules
- Optional versus nullable field semantics
- Runtime validation boundary
- Forbidden patterns (`any`, local casts of IPC payloads, duplicated contract types)

The frontend spec should reference the backend comment philosophy without duplicating the entire backend document.

## Validation Strategy

Because most code changes are comments and TypeScript contract alignment:

- Run `npm run lint`
- Run `npm run build`
- Run Rust formatting/check commands only if Rust source files change:
  - `cargo fmt --check` in `src-tauri`
  - `cargo test` or targeted tests if non-comment Rust changes occur

## Rollback Shape

Changes are low-risk and should be revertible by file:

- Spec update can be reverted independently.
- Frontend type comments/field additions can be reverted per `src/types/*.ts`.
- Backend comments can be reverted per Rust file.

No schema, migration, dependency, or generated asset changes are expected.
