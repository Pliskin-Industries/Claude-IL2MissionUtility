# 0001. Store every file byte-for-byte in git

- **Status:** Accepted
- **Date:** 2026-09-23
- **Decided by:** Claude, when the working copy was created

## Context
The app's tests compare generated `.Group` files with saved fixtures. The game writes these files with CRLF line endings, and the serializer reproduces them exactly.

## Decision
`.gitattributes` is `* -text`, so git never converts line endings.

## Rejected
- **Git's default line-ending conversion.** A checkout on another machine could rewrite the fixtures and fail the byte-for-byte tests.

## Consequences
- Do not remove or narrow `.gitattributes`.
- Text files keep whatever line endings they were written with, so diffs can show whole-file changes if an editor rewrites them.
