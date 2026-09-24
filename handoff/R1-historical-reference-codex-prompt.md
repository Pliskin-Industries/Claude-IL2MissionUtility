<task>
Build a sourced historical reference for template authors of this app (IL-2 Mission Utility, Korea 1950–53 map). Write ONE new file:

  docs/historical-reference/korea-1950-53-unit-reference.md

Purpose: when someone builds a fighter pack, ground-unit template, or ship template in this app, they check this file for historically plausible unit sizes, compositions, formations, spacings, frontages, and altitudes. Every number in the file must come from the provided source texts. Nothing is to be recalled from memory.

Inputs (read-only; they are gitignored, so they are NOT in your clone — read them from this absolute path):
  C:\Claude\IL2MissionUtility\Claude IL2Mission Utility\docs\historical-reference\sources\
  - SOURCES.md lists each source: ID (S1, S2, …), title, origin URL, and the .txt file holding its text.
  - SOURCES.md records how each .txt was made: `pdftotext -layout`, or Windows OCR for the scanned S7 and S9. OCR text keeps no layout, so tables may be scrambled; use a table value only when the row and column are unambiguous. In every .txt, pages are separated by a form feed (\f), so PDF page N is the text after the (N-1)th \f. Cite PDF pages this way.
  - Some sources are unavailable (S8 and the DTIC studies; see the end of SOURCES.md). Do not cite them. Record the coverage they would have given under Gaps. When the source also prints paragraph numbers (field manuals use "par. 45" / "45."), cite those too.
  - OCR quality varies. If a number is illegible or ambiguous in the OCR, do not use it; list it under Gaps.

App context to read (read-only) so the reference fits what the app can build:
  - README.md and USER_MANUAL.md (modes: Fighter Pack, Template Builder, Army Generator, Map, Exclusive Activation)
  - TemplateExamples/FormationTypes.Group and TemplateExamples/VehicleFormationTypes.Group (the formation types the game offers)
  - TemplateExamples/ModelTypes.Group (the plane/vehicle/ship/train models available)
</task>

<structured_output_contract>
The Markdown file has these sections, in this order:

1. **How to use this file.** 5 lines at most. Conventions: metres first with the source's original unit in brackets (e.g. "137 m [150 yd]"); citation format `[S3 p.42, par. 58]`; "—" means not found in the sources.
2. **Air — UN/US.** Fighter element, flight, and squadron sizes; formation shapes and spacing; altitude stacking and patrol layout; escort arrangement for fighter-bombers and B-29s. Tables.
3. **Air — Communist (Soviet 64th IAK / Chinese / North Korean).** Group sizes, formation shapes, altitude layering, and how units rotated through sorties, as far as the sources say.
4. **Ground — US/UN.** Tables for: rifle platoon, company, battalion, and regiment (personnel, key weapons, vehicles); tank platoon, company, and battalion (tank counts, formations such as column, wedge, line, and echelon, with intervals); field artillery battery (gun count, position layout and spacing); AAA automatic-weapons units (weapon counts, siting distances); march-column intervals; defensive frontages and depths.
5. **Ground — Chinese (CCF) and North Korean (KPA).** Organization at division, regiment, and battalion level with weapon and vehicle counts, plus attack and defense formations, as far as the sources say.
6. **Naval.** Carrier task-force composition, screen arrangement and distances, bombardment and blockade group composition, patrol and escort arrangements.
7. **IL-2 template mapping.** A table with columns: historical unit | size/composition | nearest IL-2 models (names exactly as they appear in ModelTypes.Group) | suggested game formation type (from FormationTypes/VehicleFormationTypes) | spacing to set | citation. Mark the model and formation columns "suggestion". They are editorial choices, not historical claims. If ModelTypes.Group has no suitable model, write "no model".
8. **Template checklists.** One short checklist per app mode (Fighter Pack, Template Builder air, Army Generator armor / artillery / infantry / supply / ship / train). Each item is cited.
9. **Gaps.** What the sources do not cover, contradict each other on, or cover only in illegible OCR. You MAY name well-known facts here that the sources lack, but each must be labeled "UNSOURCED — verify" and must never appear in sections 2–8.
10. **Sources.** The S# list copied from SOURCES.md (title and URL).

Your final chat message must contain: 1) path of the temp clone and branch name, 2) commit hash, 3) number of rows per section 2–8, 4) the 10 citations you consider most important, each with the exact source line quoted, so the reviewer can spot-check them, 5) the gaps you consider most serious.
</structured_output_contract>

<citation_rules>
- Every row or bullet that states a number, a unit composition, a formation shape, or a spacing carries at least one citation to an S# that exists in SOURCES.md.
- The cited page must actually contain the claim. Before you commit, re-open each cited page and confirm it. Delete any row you cannot confirm and list it under Gaps.
- If two sources disagree, show both values with both citations and note the conflict. Do not average them or pick one silently.
- Period matters: tag each row with the date or edition of its source (e.g. "FM 7-40, 1950"; "CCF, 1952"). Do not present a 1953 organization as a 1950 one.
- Paraphrase. Direct quotes in the file are at most 25 words each and used only where the exact wording matters.
- Do not use your own training knowledge for any value in sections 2–8.
</citation_rules>

<default_follow_through_policy>
Do not stop to ask routine questions. If a source is thin on a topic, write what it supports, add the rest to Gaps, and continue. If a section has no support at all, keep the heading and write one line: "No coverage in available sources — see Gaps."
</default_follow_through_policy>

<verification_loop>
Before committing:
1. Script-check that every `[S#` citation references an ID that exists in SOURCES.md and a page number no larger than that source's page count (count of \f + 1). Use a throwaway script outside the repo. Do not commit it.
2. Re-open 100% of cited pages for sections 7 and 8, and at least 30% of the other citations chosen at random. Fix or remove anything that fails.
3. `git status` in your clone must show only the one new file.
This change is documentation only, so `cargo test` is not required. Do not modify any file that would require it.
</verification_loop>

<action_safety>
Files in scope, create only: docs/historical-reference/korea-1950-53-unit-reference.md
Do not touch: everything else, including src/, TemplateExamples/, References/, assets/, USER_MANUAL.md, README.md, HANDOFF.md, .gitignore, .gitattributes, Cargo.*, and the sources/ folder (read-only input).
No network access is needed or allowed. Work only from the source texts.
</action_safety>

<git_protocol>
The Codex sandbox cannot write the primary clone's .git. So:
1. If `git -C "C:\Claude\IL2MissionUtility\Claude IL2Mission Utility" status --porcelain` shows modified tracked files, stop and report. Untracked ignored files are fine.
2. `git clone "C:\Claude\IL2MissionUtility\Claude IL2Mission Utility" <your temp dir>` and work there.
3. `git checkout -b codex/historical-reference`, create the file, commit with the message "docs: sourced Korea 1950-53 unit reference for template authors".
4. Do not push, do not commit to main, and do not write to the primary clone.
</git_protocol>
