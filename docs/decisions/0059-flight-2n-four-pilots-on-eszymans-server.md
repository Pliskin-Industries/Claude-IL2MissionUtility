# 0059. Flight 2N: up to four pilots on both sides, hosted on eszyman's server

- **Status:** Accepted
- **Date:** 2026-11-02
- **Decided by:** the user, 2026-11-02 ("Let's generate the mission so I can send it to Eszyman. Test should be doable tomorrow"; second pilot: "Eszyman, and 2 others. So we can test up to 4 as needed"; server: "Eszyman's server"; sides: "Why not NATO and DPRK?"; build: trace and clean). Claude proposed the layout below.

## Context

- Flight 2N (step 12f) was scoped on 2026-10-31 (session note `2026-10-31-p14-step12-scope.md` §3) as two human pilots on the user's own dedicated server: two areas on one side, a threshold half, and a Zone Out half.
- Since then, 0050 D replaced the copies' Zone Out with a player count per area, so the second half now tests the count.
- The user has three other pilots for 2026-11-03, including eszyman (the upstream author), who will host it.
- Until now nothing was to go to eszyman before Air Tasking had a working concept and UI. The user now judges that bar met and will send him the mission themself.

## Options

**A. Two pairs, one per side, on separate areas 85 km apart (chosen).** A DPRK pair over a NATO area and a NATO pair over a DPRK area fly the same cues at the same time.
- Each pair tests the N = 2 threshold and the count on its own area.
- The NATO pair is the first human NATO player to start a DPRK sortie, which no flight has covered.
- The pairs leave in opposite directions and never enter each other's circles, so the halves read independently.
- With 3 pilots, one side has a single pilot (expect no start there); with 2, they fly the NATO pair.

**B. Two pilots on one side, as scoped.** Rejected: it leaves two pilots idle and the NATO player side unflown.

**C. Four pilots on one area (N = 3 or a busy area).** Rejected for now: a busy area (four entries) is where 0058's known race lives, but provoking it needs a removal within about 12 s of `CLEAR`. That is hard to cue by hand, and a failed attempt would cloud the basic two-player result. It can be a later flight.

**Build:** a trace build to fly (breadcrumbs, read with Replay) and a clean build of the same inputs to share. Rejected: a trace build only (it is cluttered to show), and a clean build only (the logs could not be read against the wiring).

## Decision

**A**, with both builds. The run sheet is `handoff/sessions/2026-11-02-p14-flight-2n.md`. The files are frozen in `handoff/p14-probes/flight-2n/`.

## Consequences

- The user, not Claude, sends eszyman the mission files and the run sheet. Claude still sends nothing upstream (issues, comments, patches) without the user saying so.
- Step 11 item 5 (load time and tick rate) is measured on eszyman's server, not the user's.
- Flight name 2N and log folder `real_60` are taken by this flight.
- No product code or Generate output changes.
