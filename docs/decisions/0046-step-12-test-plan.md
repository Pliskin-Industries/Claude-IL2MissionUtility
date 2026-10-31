# 0046. Step 12 test plan: AI stand-ins first, two human pilots last

- **Status:** Accepted
- **Date:** 2026-10-31
- **Decided by:** the user, 2026-10-31 ("Approve plan"), on Claude's scoping of P14 step 12 (`handoff/sessions/2026-10-31-p14-step12-scope.md`)

## Context

Plan §8 "Step 12" (written 2026-09-27) put everything that needs two or more players into one piloted probe flight with two or three people (12b: cells T-d and T-b), after an editor task for the user (12a). Since then:

- **12a is done.** The user saved `TemplateExamples/Reference/ComplexTrigger_reference.Group` from the editor on 2026-10-12. Step 4's threshold and the probes already clone it. Only the round-trip test is missing.
- **Risk e / E6 is answered:** a fired `Dropcount 0` counter fires again after `MCU_ModifierSetVal` (flight 0). `CounterResetImpl::Ring`, its fallback, is not needed.
- **Risk p / J5 is answered:** a re-triggered running timer restarts (flight 0; flight 1M on game 1.005).
- **Cell T-d's "Confirmed if" is met without a second player.** In flights 1N and 1D, a `Country` filter excluded the other side, and the trigger reported Entered and Left once per aircraft, for AI and for a player. Flight 2J showed that a switched-off trigger reports nothing, and that switching a trigger on does not report a plane already inside. Flight A1 showed that a player who spawns inside is not reported.
- **Risk b (Closer 0 with several watched planes) is still open,** but AI aircraft can stand in for players. Check zones fire for AI (flight 0, decision 0008), and Zone Out fired when its last watched plane died (A1).
- The user wants tests a single tester can fly, with multi-player last (U14, U16).

## Decision

1. **12a:** no editor work for the user. Claude adds `complex_trigger_reference_round_trips`. No fuller editor reference (every event slot) unless the user asks for one: the N ≥ 2 design uses only Entered (58).
2. **Order:** 12c (threshold for N ≥ 2, tests first) → **flight 2M** → 12d (Zone Out, by 2M's result, decision 0048) → 12e (UI, manual) → **flight 2N** (12f).
3. **Flight 2M:** no pilot, on this PC, about 20 minutes. AI aircraft of the players' country stand in for players. Cells B test risk b: two watched planes, one leaving first by Delete, by kill, by flying out, and as the wingman of a pair. Cells D test the real N = 2 threshold cell as exported.
4. **Flight 2N:** two human pilots on the user's real dedicated server, about 30 minutes (it can be split into two halves of 15 minutes). It only confirms that players behave as the stand-ins did. Step 11 item 5 (load time and tick delay) is measured in the same session. The old T-d row "a NATO player shoots the DPRK player down inside" is dropped; A1 and cell B2 cover a death inside.
5. **Remove `CounterResetImpl`** (its only alternative was `Ring`) in its own mechanical commit, as decision 0029 removed `RandomImpl`.

## Rejected

- **§8 step 12 as written:** one piloted flight with two or three people for T-d and T-b. T-d is already answered. T-b can be answered by AI stand-ins with no pilot. People are needed only for a final confirmation.
- **AI stand-ins only, no human flight at all.** That would leave the players-equal-AI assumption unflown. The user decides later whether step 12 may merge before flight 2N, marked "AI stand-ins only" (session note, question 5).

## Consequences

- Plan §8 step 12, §3.9 and risk rows b, d and e are rewritten to match.
- The open questions for the user are in the session note, §6: who the second pilot is, whether Claude can fly as the second pilot from this PC, decision 0047, and a merge before flight 2N.
- Flight names 2M and 2N are taken. Flight 2M's logs go in the next free log folder (`real_54` today).
