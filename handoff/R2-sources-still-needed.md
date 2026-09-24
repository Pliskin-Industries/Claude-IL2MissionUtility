# R2: sources still needed for the historical reference

Status 2026-09-23. The reference is `docs/historical-reference/korea-1950-53-unit-reference.md`, and its section 9 lists the gaps in detail. The sources it already uses are in `docs/historical-reference/sources/SOURCES.md` (S1–S10).

**How to add a source:**
1. Save the PDF into `docs/historical-reference/sources/` as `S<n>_<short-name>.pdf`.
2. Make a text file. If the PDF has a text layer, run `pdftotext -layout`. If it is a scan, run the Windows OCR script (below).
3. Add a row to `SOURCES.md`.
4. Extract with the same rules: every value cited to a PDF page, with a quoted evidence line, and checked by script.

The scratch tools used for R1 were in a session temp folder. If they are gone, rebuild them:
- `page.py`: prints PDF page N, where page N is the text after the (N-1)th form feed.
- `verify.py`: checks each Evidence quote against its cited page.
- `winocr.ps1`: renders each page with Windows.Data.Pdf and OCRs it with Windows.Media.Ocr, writing a form feed between pages.

**What "Lead" means below.** A lead is a place to look, and none has been verified. Only the rows marked **verified URL** were fetched or confirmed.

## Priority 1: blocks templates we want to build

| # | Source | Fills which gap | Where to look | Notes |
|---|---|---|---|---|
| 1 | **FM 6-140, *The Field Artillery Battery*** (1950 or 1951 edition) | Guns per battery, distance between guns, battery front and depth, distance to the fire direction centre and observation post. Blocks every artillery template. | Lead: HathiTrust (US government works are public domain), the Combined Arms Research Library (CARL) digital library, other archive.org uploads. | archive.org's "FM 6-140" in `KoreanWarFieldManuals` is really FM 6-120. The same collection has **FM 6-20, *Artillery Tactics and Technique*** (14.5 MB, **verified URL**: `https://archive.org/download/KoreanWarFieldManuals/FM%206-20%20(%20Artillery%20Tactics%20and%20Technique%20).pdf`) and **FM 6-75, *105mm Howitzer M2 Series Towed*** (8.3 MB, same collection). Either may cover battery positions. Try FM 6-20 first; it needs the user's OK to download. |
| 2 | **FM 17-32, *Tank Company*** and/or **FM 7-35, *Tank Company, Infantry Regiment*** | Tanks per platoon and company, platoon formations (wedge, vee, line, echelon) and tank-to-tank intervals. FM 17-33 refers readers to these. | Lead: HathiTrust, CARL. | Not in the archive.org collection (checked 2026-09-23). |
| 3 | **Soviet 64th Fighter Aviation Corps report, July 1953** (Wilson Center Digital Archive) | MiG unit organization, pairs-and-fours doctrine, Soviet formation spacing, the Communist view of Sabre tactics. | **Verified URL:** `https://digitalarchive.wilsoncenter.org/document/report-64th-fighter-aviation-corps-soviet-air-forces-korea` | The host would not resolve from this machine by curl, web fetch or the built-in browser. Try the user's own browser or another network, then save as HTML or PDF. |
| 4 | **DTIC studies**: ADA177788 *MiG Operations in Korea*; ADA440091 *MiG Alley: The Fight for Air Superiority*; ADA340611 *No More Bad Force Myths* (regimental combat) | MiG tactics and spacing, air-superiority detail, how regiments actually fought. | **Verified URLs:** `https://apps.dtic.mil/sti/tr/pdf/ADA177788.pdf`, `https://apps.dtic.mil/sti/tr/pdf/ADA440091.pdf`, `https://apps.dtic.mil/sti/pdfs/ADA340611.pdf` | DTIC returns 403 to scripts. The user must download them in a browser. They were unavailable to the user on 2026-09-23; retry later. |

## Priority 2: fills known gaps in existing sections

| # | Source | Fills which gap | Where to look | Notes |
|---|---|---|---|---|
| 5 | **DA Pam 30-51, *Handbook on the Chinese Communist Army*, 30 September 1952 edition** | Chinese tables of organization during the war (squad to division weapon counts). Our S6 is the 1960 edition, which describes the army after the 1955 reorganization. | Lead: HathiTrust, CARL, Stanford SearchWorks (record found in search), military-book dealers. | Check the title page date. Don't trust file names. |
| 6 | **CINCPACFLT Interim Evaluation Report No. 1, Vols. II–XVI**, and **Reports No. 2–6** | Task Force 77 screen make-up and distances, blockade patrol patterns, gunfire-support stations, amphibious group make-up, mine countermeasures. | Lead: the NHHC online reading room (Vol. I came from there), the Naval War College archives (chapter records found in search), Gale (a paid collection). | Vol. I has only 28 summary pages. |
| 7 | **Handbook on the North Korean Army**, or Far East Command's **"History of the North Korean Army"** (31 Jul 1952) | North Korean division and armored-brigade organization, and T-34 counts for June 1950. | Lead: US Army Center of Military History, CARL. | Search results named the Far East Command document but it has not been located. |
| 8 | **USAF fighter tactics documents** (e.g. 4th / 51st Wing tactics pamphlets, *Fighter Weapons Newsletter*) | Distances inside a fingertip or fluid-four flight, element spacing, speeds in knots. | Lead: Air Force Historical Research Agency (AFHRA), Air University. | None found yet. Futrell gives only time intervals and Mach numbers. |

## Priority 3: nice to have

| # | Source | Fills which gap |
|---|---|---|
| 9 | Anything on **enemy small craft, minelayers and sampans** | Eastern ship templates (`sampan`, `seiner-gunboat`, `pinnace-gunboat` are in the catalog). None of the current sources covers them. |
| 10 | **Communist searchlight and flak layout** at a named target (Pyongyang, Sui-ho, the Sinanju bridges) | Actual positions. We now have only counts and "along the shoran approaches". |
| 11 | **Rail operations** (train length, anti-aircraft cars, tunnel routine) | Train templates. There are only fragments now (AA on flat cars; trains running tunnel to tunnel). |
