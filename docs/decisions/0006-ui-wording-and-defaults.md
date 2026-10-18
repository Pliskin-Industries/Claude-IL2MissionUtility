# 0006. UI wording and defaults

- **Status:** Accepted
- **Date:** 2026-09-24
- **Decided by:** the user, during the UI redesign

## Context
The UI redesign (`docs/ui-redesign/README.md`) left several choices to the user.

## Decision
- User-facing text says **DPRK / NATO**, never Eastern / Western. Names written to or matched in `.Group` files keep "Eastern" and "Zone IN".
- Skill level 3 reads **Veteran** (Plain / Low / Normal / Veteran / Ace).
- **Template translation sidecars are on hold.** The Army Generator's warning stays for now.
- The redesign is presentation only: for the same inputs, Generate writes the same `.Group` files as before.

## Rejected
- **Renaming the sides inside `.Group` files.** Existing files and the game match on the old names.

## Consequences
- Minimum text size is 12 px and minimum control height is 28 px; `src/ui_tests.rs` guards both.
- A UI change that alters any generated file is a bug.
