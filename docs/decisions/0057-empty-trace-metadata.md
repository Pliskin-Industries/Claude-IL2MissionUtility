# 0057. Metadata for an empty trace versus L's frozen base sidecar

- **Status:** Accepted (the current implementation)
- **Date:** 2026-11-01
- **Decided by:** Claude, reviewing Astra's fixes, within the user's approval of findings 3, 5 and 6. An empty trace holds no breadcrumbs, so there is nothing for a band or fingerprint to protect; keeping its old format keeps L's frozen base files byte-identical. The user may reopen this.

## Conflict

Finding 3 requires byte-for-byte comparison of every `P14_Standalone_L_Base.*` file. That includes the 41-byte `P14_Standalone_L_Base.trace.json`, with empty `entries` and `edges`. The restored comparison passed before production changes.

Findings 5 and 6 also ask new trace builds to record a band and a fingerprint. Adding those fields changes the empty base sidecar, even though the `.Group` and language files do not change. The instructions therefore cannot all be met literally for this one file.

## Option not taken

Allow metadata additions to the empty base trace while comparing its `entries` and `edges` exactly. Keep byte-for-byte comparisons of the base `.Group` and all language files, and separately assert the new band/fingerprint metadata. Do not change any frozen file.

## Chosen: the current implementation

Keep every base file byte-identical. Empty Generate traces retain their legacy format, so they have neither band metadata nor a fingerprint. Replay visibly labels them as providing no breadcrumb coverage and as unverifiable. Nonempty traces have both metadata fields and enforce the fingerprint.

This alternative preserves the strict file guarantee pending an answer; it leaves the empty-trace part of the metadata requirement unresolved.

## Rejected

- Modifying the frozen sidecar or silently omitting it from the restored comparison.
- Claiming an empty legacy trace has a verified fingerprint or useful breadcrumb coverage.
