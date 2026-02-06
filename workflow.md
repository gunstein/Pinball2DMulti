# Workflow

This is the operating manual for documentation and task management in this repository.

## Documentation (`docs/`)

| File | Purpose | When to update |
|------|---------|----------------|
| `goals.md` | Purpose, scope, non-goals, milestones | When scope changes |
| `design.md` | Architecture, code structure, protocols | When architecture changes |
| `decisions.md` | Key decisions with rationale | When a significant decision is made |

**Rules:**
- Update "Last updated" lines when editing.
- Keep `design.md` aligned with the actual codebase.
- Keep `decisions.md` short and explicit. One decision per section.
- Add new docs files when genuinely needed, not preemptively.

## Tasks (`work/task/`)

Tasks track discrete units of work (features, bugs, investigations).

### Creating a task

1. Copy `work/task/_template/` to `work/task/NNN-short-description/`
2. Fill in `description.md`
3. Add to `work/task/index.md`

### Working on a task

1. Set status to **Active** in `description.md`
2. Create an agent context file in `agents/` (one per agent)
3. Update the agent context file while working
4. Add a worklog entry at end of session

### Completing a task

1. Verify acceptance criteria in `description.md`
2. Set status to **Done**
3. Move from Active to Done in `work/task/index.md`

## How an agent should start work

1. Read this file (`workflow.md`)
2. Check `work/task/index.md` for active tasks
3. Open the task's `description.md` and agent context file
4. Update the agent context file while working
5. Add a worklog entry at the end of the session
6. Include a short summary of user prompts and agent responses in the worklog

## Quick reference

| What | Where |
|------|-------|
| Run client | `cd client && npm run dev` |
| Run server | `cd server && cargo run --release` |
| Run client tests | `cd client && npm test` |
| Run server tests | `cd server && cargo test` |
| Architecture | `docs/design.md` |
| Past decisions | `docs/decisions.md` |
| Active tasks | `work/task/index.md` |
