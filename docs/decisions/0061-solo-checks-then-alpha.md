# 0061. Flight 2N waits; solo flights A4 and 2T next; then an alpha that real operations test

- **Status:** Accepted
- **Date:** 2026-11-05
- **Decided by:** the user, 2026-11-05 ("Asking people to fly in circles for 20 mins is tough especially without engaging targets. I think we can leave 2N as pending test for now but solo test the others. If we don't get 2N by then let's go alpha live and test it in actual operations. Logs may catch the difference."). Flight designs by Claude.

## Context

Air Tasking is complete on `main`. What has not been seen in the game:

- **Two or more players** (flight 2N, decision 0059): the N ≥ 2 threshold and the player count with people, and load time and tick delay on a real server.
- **Needs one player or none:** a Yak copy, a DPRK-side extension, and the 20 % quiet roll (with the later jitter steps).

2N asks four pilots to orbit for about 20 minutes with nothing to fight, which is hard to arrange.

## Options

- **A. Hold the alpha for 2N.** The safest order, but it waits on four people with no date.
- **B. Fly the solo checks, then go alpha; real operations stand in for 2N if it has not flown (chosen).**
- **C. Go alpha now, with no further flights.** Ships three behaviours nobody has seen, though one PC can check them.

## Decision

**B.**

1. **Flight A4** (one NATO player, Claude flies on this PC): a Yak copy and a DPRK-side extension.
2. **Flight 2T** (no pilot): both clocks at a slot a minute for an hour, 120 slots, to count the quiet rate and the jitter steps.
3. If 2N has not flown when A4 and 2T are read: **an alpha release.** The `/promote` audit for the "shared" stage comes first, then a decision on upstream `571504a` and `fee0a79`, the version bump to 0.8.0, release notes with the unverified list, and a `v0.8-Alpha` tag. Each step needs the user's word.
4. In operations, missions are **trace builds**. The breadcrumbs are silent to players and let Replay read the server logs, so the multi-player behaviour is checked from real use.

The run sheet is `handoff/sessions/2026-11-05-p14-flights-a4-2t.md`. The files are frozen in `handoff/p14-probes/flight-a4/` and `flight-2t/`.

## Consequences

- 2N's files, run sheet and log folder `real_60` stay reserved. A4 takes `real_61` and 2T `real_62`.
- A finding in A4 or 2T that changes Generate output comes before the alpha.
- The manual's UNVERIFIED line keeps the multi-player items until 2N or operation logs answer them.
