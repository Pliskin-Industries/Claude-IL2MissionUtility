# 0056. Replay sessions and safe trace builds

- **Status:** Accepted
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Fix 1–9, and A–C too"), approving nine confirmed findings from Astra's review of `b5acb35..6f02211` (job `20261101145549-9eb2af5b`). The implementation choices below are Astra's (job `20261101163740-d6e3838d`), reviewed by Claude.

## Decisions

1. Replay groups `missionReport(timestamp)[n].txt` files by their timestamp. The newest session is selected initially; the picker appears only for multiple sessions. Run rescans the folder and preserves an existing selection. The core parser refuses mixed sessions.
2. A trace build must use a `.Group` filename, case insensitive, before any mission or sidecar write. Deriving a sidecar path always produces a different path; explicit sidecar reads remain supported.
3. L's base output retains the file-set check and byte comparisons for every frozen base file. Its Air output is compared against the current A3 build without introducing another golden file.
4. Discarded malformed lines produce an incomplete-report notice, count, and first three file:line locations. A session with no valid records is an error. Empty traces explicitly offer no breadcrumb coverage.
5. Map keeps its existing grid exactly. Group Generate uses an X offset of 40,000 m; standalone Air uses 80,000 m. Each band's 30,000 m width is disjoint, including subsequent rows. New nonempty sidecars record the source band. L's A3 comparison adjusts only the breadcrumb X offset before its existing index-normalized comparison.
6. New nonempty Generate sidecars carry FNV-1a 64 of the exact written `.Group` text, including line endings. Replay refuses mismatches. Legacy sidecars keep structural validation and show a visible cannot-verify note. The empty-sidecar requirement conflict is recorded separately in 0057.
7. Without enough calibration, seconds are `(T - 3) / 50`. Fitted calibration remains unchanged.
8. Instrumentation checks its entire index allocation, including the resulting allocation cursor, before mutating the tree or trace map. Every Generate caller propagates failure to the status bar before the save dialog or file writes.

## Rejected

- Merging sessions or sorting all chunks together: an earlier firing can hide a later run's dead link.
- A cached discovery result or disabling Run for an initially empty folder: newly written chunks would stay invisible.
- Accepting arbitrary trace output extensions: the mission can be overwritten by its own sidecar.
- Moving Map breadcrumbs or changing ordinary Generate: the user requires existing mission and language bytes to remain unchanged.
- Replacing the existing trace parser, adding JSON/hash crates, or introducing new Air golden files: unnecessary for the approved fixes.
- Treating malformed or empty evidence as a successful complete report, or silently accepting an unverifiable legacy sidecar.
- Updating any frozen probe input/output or running ignored writers: expressly prohibited.

## Evidence

See `handoff/sessions/2026-11-01-review-fixes-replay-trace.md` for regression names, validation results, and the one outstanding constraint question.
