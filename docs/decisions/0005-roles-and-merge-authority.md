# 0005. Fable plans and reviews, Astra builds, the user merges

- **Status:** Accepted
- **Date:** 2026-09-27
- **Decided by:** the user (P14 decisions U11, U19, U23)

## Context
Two models work on this repo through the `codex-delegation` skill. Early runs silently used the wrong model, because the Codex CLI falls back to its default when none is named.

## Decision
- **Claude (Fable role)** plans, scopes each step, writes the delegation prompt, reviews every diff, re-runs build and tests, and commits. It writes code only with the user's permission for that task.
- **GPT-6 Astra, through the Codex broker,** writes the code, one step per delegation, at `max` effort (`ultra` for reviews).
- **The user** decides every push and every merge to `main`.

## Rejected
- **A separate Opus session as the builder** (the first plan). Replaced by Astra.
- **A further full plan review before the build.** Fable re-reads each step's section when it scopes that step.

## Consequences
- Every delegation passes `model` explicitly, and the job's `output.log` must name `gpt-6-astra`.
- Delegation is same-host: Astra works in its own worktree and branch `codex/p14-<step>`; Fable merges into `claude/p14-air-tasking` with `--no-ff`. Steps: `handoff/P14-air-tasking.md` §10.
- Codex install notes for a new machine: `handoff/archive/HANDOFF-through-2026-10-08.md` §2.
