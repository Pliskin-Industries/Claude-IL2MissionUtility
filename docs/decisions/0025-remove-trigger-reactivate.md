# 0025. Remove the TriggerReactivate presence check from the code

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user, 2026-10-18 ("Accept decision 0025: remove TriggerReactivate, set the walker trigger default to Silent, freeze flight 2J"), on Claude's reading of flight 2J

## Context

Decision 0024 made `ZonePulse` the threshold's default after flight 2J showed that a switched-on Complex Trigger does not report a plane already inside. `PresenceCheckImpl::TriggerReactivate` is still built, tested and selectable, and the walker's default hypothesis (`TriggerActivation::ReportsInside`) models the behaviour the game does not have.

## Decision

1. Remove `PresenceCheckImpl` and the `TriggerReactivate` branch: `threshold::build_side` builds the `ZonePulse` cell only, and its tests lose the `TriggerReactivate` variants (the 18-MCU structure, `SEEN`, and the walker tests that depend on k1). Delegated in one job, `mod.rs` included: the type cannot be removed in a commit that compiles without its users.
2. Set the walker's Complex Trigger default to `TriggerActivation::Silent`, with a comment citing flight 2J, and keep `ReportsInside` only if a test still needs it.
3. Flight 2J's probe keeps building its cells as flown (it would call the old variant): freeze flight 2J like flights 2E and 2G, so its flown files stay in the repository but are no longer regenerated.

## Rejected

- **Keep it as a selectable strategy.** It is known not to work, so it only adds code, tests and a wrong walker default (the user's rule: the cleanest design).

## Consequences

- One code path for the threshold; the walker models the game as flown.
- The flown-file count drops from 56 to 52 when flight 2J is frozen.
