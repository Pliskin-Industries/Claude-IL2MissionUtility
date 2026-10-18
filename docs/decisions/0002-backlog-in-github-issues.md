# 0002. The backlog lives in GitHub Issues

- **Status:** Accepted
- **Date:** 2026-09-26
- **Decided by:** the user

## Context
The backlog was a table in `HANDOFF.md`. It grew with every session, and a collaborator could not see or comment on single items.

## Decision
The backlog is GitHub Issues on the fork (`Pliskin-Industries/Claude-IL2MissionUtility`). It was imported as #1 to #27 on 2026-09-26. Design detail stays in `handoff/*.md`, and the issues link to it.

## Rejected
- **Keeping the table in the handoff file.** It could not be assigned, labelled or closed.

## Consequences
- Close an issue when its work merges.
- Labels in use: `terrain`, `template-builder`, `verify-in-game`, `proposal`, `deferred`, `blocked`, `long-term`, `author-todo`, `research`, `airfield`, `cleanup`.
- The old phase table (P1 to P14) is history: `handoff/archive/HANDOFF-through-2026-10-08.md` §4.
