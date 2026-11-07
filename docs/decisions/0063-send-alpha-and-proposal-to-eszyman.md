# 0063. Send eszyman a v0.8-Alpha build and the Air Tasking proposal now, in one issue

- **Status:** Accepted
- **Date:** 2026-11-06
- **Decided by:** the user, 2026-11-06 ("can we start the process to send what we have to eszyman to test it"; then "Merge first", "Alpha release v0.8-Alpha", "Yes, with the build"). Options by Claude.
- **Supersedes:** [0060](0060-air-tasking-beside-exclusive-activation.md) in its timing only ("sent after the alpha has logs from real operations"). Its direction (two modes, the replace-it explanation) stands.

## Context

Flights A4 and 2T passed (decision 0061). Flight 2N has not flown. The user wants eszyman to test what the fork has now.

## Options

- **Delivery:** a tagged alpha release `v0.8-Alpha` (chosen; a public exe on the fork), or a workflow build with no release.
- **Upstream first:** merge his `571504a` and `fee0a79` first (chosen, decision 0062), or send a build without them.
- **Proposal:** send the rewritten proposal with the build (chosen), or the build alone and the proposal after real-operation logs (0060's plan).

## Decision

1. Merge upstream (0062).
2. Run the `/promote` audit for the "shared" stage, since another person will use the build.
3. Bump to 0.8.0, write the release notes with the UNVERIFIED list, and push the `v0.8-Alpha` tag, which starts the release build.
4. Update `handoff/upstream-air-tasking-proposal-draft.md` with A4, 2T and test steps.
5. Post one issue on `eszyman/IL2MissionUtility` with the build link and the proposal.

Steps 3 (the tag push) and 5 (the post) each happen only at the user's word, after the user has read the text.

## Consequences

- Real-operation logs come after the proposal, not before it. The proposal says what is unverified.
