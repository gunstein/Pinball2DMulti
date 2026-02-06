# Agent Repo Setup Guide

This guide describes how an agent should initialize a repository for durable, low-friction collaboration. It is language-agnostic and focuses on workflow, documentation, and task tracking.

## Goals
- Make long gaps between sessions easy to recover from.
- Preserve context for every task and every agent.
- Keep intent, design, and decisions visible and current.

## Required Structure
Create the following structure at the repository root:

```
README.md
workflow.md
agent_repo_setup.md

docs/
  README.md
  goals.md
  design.md
  roadmap.md
  requirements.md
  decisions.md

work/
  task/
    README.md
    index.md
    _template/
      description.md
      worklog.md
      agents/
        README.md
        agent-context.md
```

## Core Files and Their Purpose
- `README.md`: Minimal entry point pointing to `workflow.md`.
- `workflow.md`: The operating manual for docs and task workflow.
- `docs/goals.md`: Purpose, scope, non-goals, milestones.
- `docs/design.md`: Current-state architecture and flow.
- `docs/roadmap.md`: Vision and forward-looking plans.
- `docs/requirements.md`: Constraints and requirements (as needed).
- `docs/decisions.md`: Key decisions to prevent re-litigation.
- `work/task/index.md`: Quick list of active/blocked/done tasks.
- `work/task/README.md`: Task naming and template conventions.
- `work/task/_template`: The canonical structure for new tasks.

## Task Template Requirements
`work/task/_template/description.md` must include:
- `Status: Active | Done | Blocked`
- Summary, Context, Goals
- Acceptance Criteria as a checklist
- Non-Goals and Notes/Links
- Optional sections (Open Questions, Dependencies, Risks, Validation, Definition of Done)

`work/task/_template/worklog.md` must include dated entries and a brief prompt/response summary.
Optional but recommended: include a short example entry to model the format.

Agent context is **required** for each task:
- Each agent keeps a rolling file in `work/task/<task>/agents/<agent-id>.md`.
- Start from `agents/agent-context.md`.

## Conventions
- Use `NNN-short-description` for task folder names.
- Update “Last updated” lines in docs when editing.
- Keep `docs/design.md` aligned with the actual codebase.
- Keep `docs/roadmap.md` forward-looking and free to change.
- Keep `docs/decisions.md` short and explicit.

## Minimal README.md
Use a minimal top-level README like this:

```md
# Project

See `workflow.md` for how documentation and tasks are organized.
```

## How an Agent Should Start Work
1. Read `workflow.md`.
2. Check `work/task/index.md` for active tasks.
3. Open the task’s `description.md` and the agent context file.
4. Update the agent context file while working.
5. Add a worklog entry at the end of the session.
6. Include a short summary of user prompts and agent responses in the worklog entry.
