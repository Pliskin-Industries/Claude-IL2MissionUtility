# 0004. Adopt upstream's 1500 m airstart; drop auto altitude

- **Status:** Accepted
- **Date:** 2026-09-27
- **Decided by:** the user (P14 decision U15)

## Context
On 2026-09-23 this fork built "auto altitude": new Template Builder planes started at 50 % of their service ceiling, and wingmen matched their lead. The creator then changed the same code upstream (`c1d88c3`): every airstart is at 1500 m. The P14 baselines had to be recorded on one behaviour or the other.

## Decision
Merge upstream `c1d88c3` in full (on `main` as `a34a00e`). Airstart is 1500 m for every aircraft (`template::AIR_START_ALTITUDE_M`); the slider still changes one plane.

## Rejected
- **Keeping the fork's auto altitude.** It would diverge from the creator's behaviour and make every later upstream merge harder.

## Consequences
- Removed: the "Auto altitude" checkbox, match-lead, `model_spec::auto_altitude_m`, and the Fighter Pack reinforcement timer.
- The record of what auto altitude was: `handoff/archive/HANDOFF-through-2026-10-08.md` §5.
- General rule this set: when upstream and the fork change the same behaviour, take upstream's unless the user says otherwise.
