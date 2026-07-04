# Multi-Agent Collaboration Guide

> **Purpose**: Define a stable handoff protocol for Kairos when Codex and Claude Code work on the same repository through separate sessions and separate model stacks.

---

## Why This Guide Exists

Kairos is not using a single orchestrated multi-agent runtime.

The actual workflow is:

- `Codex + GPT` handles planning, architecture, review, and wrap-up
- `Claude Code + DeepSeek` handles much of the implementation work
- The developer manually switches between the two tools

This creates three recurring risks:

1. **Context drift** — one side keeps working from chat memory while the other side has already changed the task artifacts
2. **Overlapping edits** — both tools touch the same task or the same files without a clear baton pass
3. **Weak handoff quality** — implementation finishes, but review receives no precise summary of what changed, what remains risky, or what was actually validated

This guide exists to make the manual handoff explicit and repeatable.

---

## Core Rule

> **Chats are transient. Task files are the source of truth.**

When work moves between Codex and Claude Code, neither side should rely on the prior chat transcript as the authoritative state. The authoritative state must live in `.trellis/tasks/<task>/` and relevant `.trellis/spec/` files.

---

## Standard Role Split

### Codex / GPT

Use Codex as the default owner for:

- brainstorming and requirement discovery
- `prd.md`
- `design.md`
- `implement.md`
- architecture-sensitive decisions
- cross-layer reasoning
- code review / quality check
- spec update decisions
- final wrap-up judgment

### Claude Code / DeepSeek

Use Claude Code as the default owner for:

- executing the reviewed implementation plan
- repetitive but structured code changes
- plumbing across files once the design is already fixed
- iterative implementation under an existing `implement.md`

### Human

The human owns:

- switching tools
- deciding when baton passes happen
- resolving conflicts when artifacts and implementation diverge
- preventing simultaneous edits to the same task scope

---

## Single Source of Truth

For any shared task, the following files are the canonical state:

- `prd.md`
- `design.md`
- `implement.md`
- `research/*.md`
- `handoff.md` if present

Never treat these as optional when switching tools.

### Rule

Before continuing an existing task in another tool:

1. read the current task directory
2. read the latest planning artifacts
3. read `handoff.md` if it exists
4. inspect the current git working tree

If the chat says one thing and the task files say another, **trust the task files**.

---

## Baton-Pass Model

Kairos should use **sequential ownership**, not concurrent ownership, for a task.

Recommended pattern:

1. `Codex` creates or updates planning artifacts
2. human reviews and decides implementation should begin
3. `Claude Code` implements according to `implement.md`
4. `Claude Code` writes handoff state
5. `Codex` reviews, checks, and either requests another implementation pass or wraps up

### Hard Rule

Do not let Codex and Claude Code actively edit the same task at the same time unless the files are explicitly partitioned and the handoff is recorded first.

---

## Handoff File

Each cross-tool task should maintain:

```text
.trellis/tasks/<task>/handoff.md
```

This file is intentionally lightweight. It exists to reduce repeated repo archaeology.

### Required sections

```markdown
# Handoff

## Current Phase

- planning / in_progress / check / blocked

## Last Owner

- Codex
- Claude Code

## What Was Done

- ...

## Files Changed

- path
- path

## Validation Run

- command: result
- command: result

## Known Risks / Open Issues

- ...

## Recommended Next Owner

- Codex review
- Claude implementation
- Human decision
```

### Minimum standard

A handoff is not complete if it only says “done” or “implemented”.

It must answer:

- what changed
- where it changed
- what was verified
- what still looks risky

---

## When To Use Two Tools

Use dual-tool collaboration when **at least one** is true:

- the task spans 3+ layers
- architecture or state consistency matters more than raw typing speed
- the task needs a strong planning phase before code
- implementation is broad but mostly mechanical once the design is fixed
- final review quality matters more than implementation speed

Examples:

- Tauri plugin integration
- database migration + command + frontend wiring
- sync protocol changes
- notification scheduling
- cross-platform behavior changes

---

## When NOT To Use Two Tools

Prefer a single tool end-to-end when:

- the task is a tiny bugfix
- only one file is involved
- the work is mostly visual polish
- there is no architecture ambiguity
- handoff overhead would exceed implementation time

Examples:

- button spacing tweak
- copy text fix
- one-field type mismatch
- trivial validation patch

### Rule of thumb

If the task does not clearly justify `prd/design/implement`, it probably does not justify cross-tool execution either.

---

## No-Parallel Zones

The following situations forbid concurrent editing by Codex and Claude Code:

- both tools modifying the same task artifacts
- both tools editing the same feature files
- one tool changing design while the other already implements the old design
- one tool preparing commit grouping while the other still edits source files

If parallelism is truly needed, split into separate child tasks first.

---

## Task Split Strategy

If one request contains multiple independently testable deliverables:

- create a parent task for the shared requirement set
- create child tasks for each concrete work item

This is the only safe way to get partial parallelism without chaos.

### Good split

- parent: notification capability
- child A: planning and architecture
- child B: plugin integration
- child C: exam scheduler
- child D: pomodoro scheduler

### Bad split

- one giant task where two tools both edit the same Rust and TS files at once

---

## Recommended Kairos Workflow

### Pattern A: Planning in Codex, Implementation in Claude, Review in Codex

1. `Codex`
   - create task
   - write `prd.md`
   - write `design.md` if needed
   - write `implement.md` if needed

2. Human
   - confirm artifacts are good enough
   - start task
   - switch to Claude Code

3. `Claude Code`
   - read task artifacts
   - implement only the reviewed scope
   - update `handoff.md`

4. Human
   - switch back to Codex

5. `Codex`
   - run review/check
   - decide fix loop vs wrap-up

This should be the default workflow for medium and large tasks.

### Pattern B: Single-Agent Fast Path

If the task is tiny, skip cross-tool handoff entirely and use one agent from start to finish.

---

## Handoff Checklist

Before switching tools, verify:

- [ ] current task is correct
- [ ] task artifacts are saved to disk
- [ ] no unsaid design decisions exist only in chat
- [ ] `handoff.md` is updated if implementation occurred
- [ ] `git status` is understandable
- [ ] next owner is explicit

If any item is missing, the baton pass is not ready.

---

## Review Expectations

Codex review after Claude implementation should focus on:

- scope drift from `implement.md`
- cross-layer mismatches
- hidden regressions
- partial validation or missing tests
- frontend/backend contract mismatches
- platform-specific omissions

Do not treat “it compiles” as enough.

---

## Conflict Resolution Rule

If Codex review discovers that implementation and planning artifacts disagree:

1. stop implementation expansion
2. decide whether the artifacts were wrong or the implementation drifted
3. update the source of truth first
4. only then continue implementation

Never silently let code and task artifacts diverge.

---

## Minimal Human Operations

The human should minimize manual orchestration to these actions:

1. choose the current owner
2. switch tools
3. confirm when planning is approved
4. confirm when implementation should pause for review
5. arbitrate conflicts

The human should **not** need to repeatedly restate requirements if task artifacts are maintained correctly.

---

## Anti-Patterns

- Letting both tools “just continue from context”
- Starting implementation before planning is frozen
- Using chat history as the handoff mechanism
- Skipping `handoff.md` because the change “felt obvious”
- Asking the reviewer to infer what the implementer changed from git diff alone
- Reopening the same task in both tools without checking task files first

---

## Quick Decision Matrix

| Situation | Recommended owner |
|---|---|
| New feature, unclear scope | Codex |
| Cross-layer design | Codex |
| Broad mechanical implementation | Claude Code |
| UI polish with strong visual judgment | Codex |
| Review / check / wrap-up | Codex |
| Tiny isolated patch | One tool only |

---

## Suggested Default

For Kairos, the default should be:

- `Codex` for planning and review
- `Claude Code` for execution
- `handoff.md` for every cross-tool implementation pass
- no simultaneous edits on the same task scope

This is the lowest-friction version of multi-agent collaboration that still preserves engineering control.
