# Service ceilings of 12 Korean War aircraft

Research date: 2026-10-12. Primary documents were read for 3 of the 12 aircraft (F-51D, B-29, C-47). For 2 more (F-80C, F-84E) the figure comes from the official USAF history (Knaack 1978), read with word positions so that each value is tied to its table column. The other 7 rest on web pages that name books as their sources. Each row is labelled.
No USAF Standard Aircraft Characteristics (SAC) chart was found for the F-80C, F-84E or F-86A: archive.org holds none (title search, 2026-10-12), and the index at generalstaff.org lists none for these types.

**Status:** on 2026-10-13 the user ruled that all twelve aircraft take the "Recommended" figure, and `src/model_spec.rs` now holds them, each with its source in a comment. Where this page says "the tool" or "tool before", it means the figures the app had until then.

The column "Sim IL-2 Korea" is the practical ceiling the game's own handbook shows (`docs/game-handbook/aircraft-characteristics.md`). It is there for comparison only.

Conversion used throughout: 1 ft = 0.3048 m. The unit the source used is marked "(orig.)".

Ceiling definitions used below:

- Service ceiling (US): altitude where rate of climb falls to 100 ft/min.
- Combat ceiling (US): altitude where rate of climb falls to 500 ft/min.
- Absolute ceiling: rate of climb is zero.
- Practical ceiling (Soviet, практический потолок): rate of climb falls to 0.5 m/s (about 98 ft/min). It is the same idea as the US service ceiling.

## 1. Summary table

| Aircraft | Recommended ceiling (m) | Ceiling type and weight | Source (short) | Confidence | Tool before 2026-10-13 (m) | Sim IL-2 Korea (m) |
|---|---|---|---|---|---|---|
| F-80C-10 | 13030 | Service; weight not stated, table gives max gross 16,856 lb (42,750 ft orig.) | Knaack 1978, p. 11, F-80C column; official USAF history | Medium | 13716 | 12900 |
| F-84E | 13716 | Service; weight not stated, table gives max gross 18,000 lb (45,000 ft orig.) | Knaack 1978, p. 50, F-84E column; official USAF history | Medium | 12192 | 13700 |
| F-86A-5 | 14630 | Service; weight not tied to the figure (48,000 ft orig.) | Baugher, citing Wagner 1963 and others; secondary | Low | 15240 | 14900 |
| F-51D | 12680 | Service, 100 ft/min, 9,760 lb take-off, wing racks fitted (41,600 ft orig.) | Wright Field Memo Report TSCEP5E-1908, 15 June 1945 | High | 11278 | 12700 |
| B-29 | 10668 | Combat, 500 ft/min, combat weight 100,000 lb, maximum power (35,000 ft orig.) | USAF Characteristics Summary, B-29, 9 March 1949 | High | 10668 | 10700 |
| C-47B | 8047 | Service, 26,000 lb gross, two-speed blower working (26,400 ft orig.) | Wikipedia citing Francillon 1988; secondary | Low | 6096 | 8600 |
| MiG-15bis | 15500 | Practical; weight not tied to the figure, normal take-off 4,960 kg (m orig.) | ru.wikipedia citing Gordon 2001 and Arsenyev & Krylov 1999; secondary | Low | 15000 | 15500 |
| La-11 | 10250 | Practical, take-off weight 3,730 kg (m orig.) | airpages.ru citing Shavrov; secondary | Low | 10150 | 10300 |
| Yak-9P | 10500 | Practical, take-off weight 3,550 kg, 1947 all-metal version (m orig.) | airpages.ru; secondary | Low | 10600 | 11000 |
| Il-10 | 7250 | Practical, flight weight 6,300 kg (m orig.) | airwar.ru; ru.wikipedia citing Yakubovich 2013; secondary | Low | 6950 | 6000 |
| Tu-2S | 9500 | Practical, normal take-off 10,360 kg, ASh-82FN (m orig.) | airpages.ru citing Shavrov; secondary | Low | 10500 | 8000 |
| Li-2T | 5600 | "Ceiling", passenger Li-2 with M-62IR; weight unclear (m orig.) | ru.wikipedia; secondary | Low | 6800 | 5500 |

Read the B-29 and C-47B rows with their sections. The B-29 figure is a combat ceiling at a light weight, and the C-47B figure depends on whether the high blower works.

## 2. Sources

| ID | Title | Edition / date | URL read | How it was read |
|---|---|---|---|---|
| C1 | *Flight Tests on the North American P-51D Airplane, AAF No. 44-15342*, Memo Report TSCEP5E-1908 | Wright Field, 15 June 1945 | http://www.wwiiaircraftperformance.org/mustang/p51d-15342.html | HTML transcription of the report |
| C2 | T.O. 1F-51D-1 (formerly AN 01-60JE-1), *Flight Handbook, USAF Series F-51D* | 20 January 1954; replaces the 15 July 1952 issue | https://archive.org/details/01-60-jf-1-foi-p-51-h | archive.org OCR text, characters 0–100,000 and 200,000–300,000 of 310,621 |
| C3 | *B-29 Superfortress Characteristics Summary* | USAF, 9 March 1949 | https://archive.org/details/B29SuperfortressCS9March1949 | archive.org OCR text (one-sheet chart) |
| C4 | T.O. 1C-47-1, *Flight Manual, USAF Series C-47, A, B, D ...* | 15 March 1963 | https://archive.org/details/to-1-c-47-1-flight-manual-usaf-series-c-47 | archive.org OCR text, characters 0–100,000 and 400,000–588,882 |
| C5 | Marcelle S. Knaack, *Encyclopedia of US Air Force Aircraft and Missile Systems, Vol. 1: Post-World War II Fighters 1945–1973* | Office of Air Force History, 1978 (DTIC ADA057002) | https://archive.org/details/DTIC_ADA057002 | archive.org OCR text; tables at pp. 11, 50, 80 |
| C6 | Joe Baugher, "North American F-86A Sabre" | web page, undated | http://www.joebaugher.com/usaf_fighters/p86_2.html | HTML |
| C7 | *Самолет Ту-2 с моторами АШ-82ФН, книга 3: Описание конструкции*, 2nd ed., ed. D. P. Soloukhin | Oborongiz, Moscow, 1946 | https://archive.org/details/Tu-2-1946 | archive.org OCR text, characters 0–100,000 of 261,309 |
| C8 | airpages.ru pages: La-11, Yak-9P, Il-10, Tu-2, Li-2 | web pages, undated | https://airpages.ru/ru/la11.shtml , https://airpages.ru/ru/yak9p.shtml , https://airpages.ru/ru/il10.shtml , https://airpages.ru/ru/tu2.shtml , https://airpages.ru/ru/li2.shtml | HTML |
| C9 | airwar.ru pages: La-11, Il-10, MiG-15bis | web pages; Il-10 page dated 2018 | https://airwar.ru/enc/fww2/la11.html , https://airwar.ru/enc/aww2/il10.html , https://airwar.ru/enc/fighter/mig15bis.html | HTML; Cyrillic text arrived garbled, numbers legible |
| C10 | Wikipedia articles (Russian and English) for each type | read 2026-10-12 | URLs given in each section | HTML |
| C11 | Eagle Dynamics, "DCS: MiG-15bis" product page | web page, undated | https://www.digitalcombatsimulator.com/es/products/planes/mig15bis/ | HTML |

Not available (listed so gaps can be attributed):

- The SAC and Characteristics Summary charts are indexed at https://generalstaff.org/CDA/Aircraft.htm (B-29 list at https://generalstaff.org/CDA/Air/B-29/B-29.htm). The host returned HTTP 403 to web fetch; read in a browser, the index has bombers, attack aircraft and a few other types, and no page for the F-80, F-84, F-86, F-51 or C-47. The older host, alternatewars.com, could not be opened.
- apps.dtic.mil, media.defense.gov, nationalmuseum.af.mil and ang.af.mil returned HTTP 403. web.archive.org could not be fetched.
- No copy of T.O. 1F-80C-1, the F-84E flight handbook, or the F-86A flight handbook was found on archive.org.
- No Soviet technical description or pilot's instruction that states a practical ceiling was found for the MiG-15bis, La-11, Yak-9P, Il-10 or Li-2. No НИИ ВВС state-test report was read directly for any type.
- USAF Museum fact-sheet figures quoted below were seen only in search-result summaries of those pages, not on the pages themselves.

## 3. Aircraft

### F-80C-10 Shooting Star

Figures found:

- 42,750 ft (orig.) = 13,030 m. "Service Ceiling", F-80C column, Technical Data table, C5 p. 11. No weight is stated; the table gives max gross weight 16,856 lb. The same table gives 45,000 ft for the F-80A/B and 47,500 ft for the T-33A. Column checked by word position in the scan's text layer (see "How the Knaack tables were read"). Secondary (official USAF history; its preface says it is based mainly on USAF sources and names no document for the table).
- 46,800 ft (orig.) = 14,265 m. "Service ceiling", P-80C/F-80C, English Wikipedia, https://en.wikipedia.org/wiki/Lockheed_P-80_Shooting_Star . It cites L. K. Loftin, *Quest for Performance*, NASA SP-468, Appendix A, Table V, and R. J. Francillon, *Lockheed Aircraft Since 1913* (1982). The page lists gross weight 12,200 lb but does not tie it to the ceiling. Secondary. Search summaries of the USAF Museum F-80C fact sheet give the same 46,800 ft.
- 42,750 ft also appears on the Pima Air & Space Museum page (https://pimaair.org/designation/f-80c/), seen in a search summary only.

Recommended: 13,030 m (42,750 ft), medium confidence. Reason: it is the only figure that comes from an official USAF publication and is tied to the F-80C by its table column. The 46,800 ft figure is 4,050 ft higher; that is the size a weight difference would produce (a clean aircraft near 12,200 lb against one at take-off weight with tip tanks), but no source read confirms that explanation, and it is my inference.

Against the two existing sets: the tool's 13,716 m is exactly 45,000 ft, which is Knaack's F-80A/B figure, not the F-80C figure. The sim's 12,900 m is 130 m below Knaack's F-80C figure.

### F-84E Thunderjet

Figures found:

- 43,240 ft (orig.) = 13,180 m. "Ceiling", USAF Museum F-84E fact sheet (https://www.nationalmuseum.af.mil/Visit/Museum-Exhibits/Fact-Sheets/Display/Article/196111/republic-f-84e-thunderjet/) and the FAS copy (https://nuke.fas.org/guide/usa/airdef/f-84e.htm). Seen in search summaries only. Type of ceiling and weight not stated; the sheet gives 15,227 lb loaded. Secondary.
- 45,000 ft (orig.) = 13,716 m. "Service Ceiling", C5 p. 50, table headed F-84B, F-84C/D, F-84E, F-84G. The row has three values for four columns: 40,750 ft (F-84B), none (F-84C/D), 45,000 ft (F-84E), 40,500 ft (F-84G). Column checked by word position: the 45,000 ft entry sits under the F-84E heading, with that column's 11,000 lb empty weight, 18,000 lb max gross weight and 739 nm combat radius. Secondary (official USAF history).
- 40,500 ft (orig.) = 12,344 m is the F-84G figure (English Wikipedia, https://en.wikipedia.org/wiki/Republic_F-84_Thunderjet , citing Thompson, *Thunderjets in Korea*, 1982, p. 51). It is not an F-84E figure.

Recommended: 13,716 m (45,000 ft), medium confidence. Reason: it comes from an official USAF publication and is tied to the F-84E by its table column. It is a round number, and the museum sheet's 43,240 ft is 1,760 ft lower; that sheet was not read directly and states no ceiling type or weight.

Against the two existing sets: the sim's 13,700 m is Knaack's 45,000 ft rounded. The tool's 12,192 m is exactly 40,000 ft and sits near the F-84G figure (40,500 ft).

### F-86A-5 Sabre

Figures found:

- 48,000 ft (orig.) = 14,630 m. "Service ceiling", F-86A-5-NA, C6. The page gives combat weight 13,791 lb and take-off weight 14,108 lb, but does not tie the ceiling to either. Its sources are books (Wagner, *The North American Sabre*, 1963; Davis 1992; Angelucci and Bowers 1987; Green 1966). Secondary.
- 49,000 ft (orig.) = 14,935 m. "Combat ceiling", F-86A, Air National Guard copy of the USAF Museum fact sheet (https://www.163atkw.ang.af.mil/About-Us/Fact-Sheets/Display/Article/452910/north-american-f-86-sabre-day-fighter-a-e-and-f-models). Seen in a search summary only. Secondary. A combat ceiling above the service ceiling is not possible under the USAF definitions, so the label on one of these two figures is loose.
- C5 p. 80, table headed F-86A, F-86F, F-86H: the "Service Ceiling" row is empty in the F-86A column. Its two values, 46,000 ft and 47,200 ft, sit under F-86F and F-86H (checked by word position). Knaack gives no F-86A ceiling.

Recommended: 14,630 m (48,000 ft), low confidence. Reason: it is the only figure labelled as a service ceiling for the A-5 by name.

Against the two existing sets: the sim's 14,900 m matches the 49,000 ft "combat ceiling" figure. The tool's 15,240 m is exactly 50,000 ft and matches nothing found. The sim is closer to the recommended value (270 m against 610 m). No primary source was read.

### F-51D Mustang

Figures found:

- 41,600 ft (orig.) = 12,680 m. Service ceiling, C1. Aircraft: P-51D-15, AAF No. 44-15342, V-1650-7. Take-off weight 9,760 lb with full ammunition, 184 gal wing fuel and 25 gal in the fuselage tank. One bomb rack on each wing, no stores. War emergency power, 67 in Hg. The climb table gives 100 ft/min at 41,600 ft, so this is a service ceiling by the standard definition. Time to that height: 28 minutes.
- 42,400 ft (orig.) = 12,924 m. Absolute ceiling, same report and condition.
- 41,900 ft (orig.) = 12,771 m. "Service ceiling", English Wikipedia (https://en.wikipedia.org/wiki/North_American_P-51_Mustang), which lists the *P-51 Tactical Planning Characteristics & Performance Chart* among its sources. Gross weight listed as 9,200 lb, not tied to the ceiling. The chart itself was not read. Secondary.
- C2, the 1954 Flight Handbook, has no ceiling figure in the portions read. Its climb charts (figures A-5 and A-6) list gross-weight bands and altitudes to at least 30,000 ft; the OCR of the chart values is not legible.

Recommended: 12,680 m (41,600 ft), high confidence. Reason: it is a measured figure from an official flight-test report, with weight, configuration and power stated. The report was read as an HTML transcription, not as page scans.

Against the two existing sets: the sim's 12,700 m matches (20 m difference, rounding). The tool's 11,278 m is exactly 37,000 ft; no source for 37,000 ft was found, and no definitional difference explains a 4,600 ft gap.

### B-29 Superfortress

Figures found, all from C3. The chart is for the B-29 with four R-3350-57/-57A engines. Weights: empty 71,500 lb, combat 100,000 lb, take-off 140,000 lb. Its notes give the basis as the Pilot's Handbook dated 13 January 1948 and NACA standard conditions.

- 35,000 ft (orig.) = 10,668 m. Combat ceiling: 500 ft/min, combat weight, maximum power.
- 23,800 ft (orig.) = 7,254 m. Service ceiling: 100 ft/min, take-off weight, normal power.
- The summary sheet gives no service ceiling at combat weight. The full SAC chart would; it was not readable (see "Not available").
- 31,850 ft (orig.) = 9,708 m. "Service ceiling", English Wikipedia (https://en.wikipedia.org/wiki/Boeing_B-29_Superfortress), citing a Boeing web page. No weight stated. Secondary.

Recommended: 10,668 m (35,000 ft), high confidence in the figure, with a caution on its meaning. It is a combat ceiling at 100,000 lb. A B-29 at take-off weight has a service ceiling of only 7,254 m. A service ceiling at combat weight would be somewhat above 10,668 m, but that number was not found.

Against the two existing sets: the tool's 10,668 m is the same figure. The sim's 10,700 m is the same figure rounded. Both therefore carry a combat ceiling (500 ft/min) at combat weight, not a service ceiling.

### C-47B Skytrain

Figures found:

- 22,000 ft (orig.) = 6,706 m. C4, Figure A4-26, "Emergency Ceiling, Standard Day, 100 ft/min Rate of Climb at METO Power", printed page A4-43/A4-44. The chart is for two R-1830-90C with high blower inoperative, and for the -90D and -92. The figure is the worked sample problem: 27,000 lb gross weight, two engines, with-skis curve. A 100 ft/min ceiling is a service ceiling by definition, here at METO power. The curve without skis was not legible in the OCR and would be slightly higher.
- C4's introduction states that aircraft with the R-1830-90C are designated C-47B, and that on some of them the high blower has been made inoperative.
- 26,400 ft (orig.) = 8,047 m. "Service ceiling", C-47B-DK, English Wikipedia (https://en.wikipedia.org/wiki/Douglas_C-47_Skytrain), citing Francillon, *McDonnell Douglas Aircraft since 1920*, Vol. I (1988), pp. 217–251. Gross weight 26,000 lb, R-1830-90C. Secondary.
- 7,070 m (orig.) = 23,196 ft. "Practical ceiling", C-47 with R-1830-93, comparison table on the airpages.ru Li-2 page (C8). This engine is not the C-47B's. Secondary.

Recommended: 8,047 m (26,400 ft), low confidence, for a C-47B with a working two-speed supercharger. If the tool models Korean War aircraft with the high blower disabled (the C-47D standard), use about 6,700 m (22,000 ft at 27,000 lb), medium confidence. The medium figure comes from a primary manual but from one sample problem on a 1963 chart read through OCR.

Against the two existing sets: the tool's 6,096 m is exactly 20,000 ft and is below both figures. The sim's 8,600 m (28,200 ft) is above both. The sim is closer to the working-blower figure (553 m); the tool is closer to the disabled-blower figure (610 m). The blower condition and the weight explain the spread between sources. They do not explain the sim's 8,600 m.

### MiG-15bis

Figures found:

- 15,500 m (orig.) = 50,853 ft. Practical ceiling. Russian Wikipedia, https://ru.wikipedia.org/wiki/МиГ-15бис , data given for MiG-15bis serial no. 105015 of 1949. It cites Gordon, *Mikoyan-Gurevich MiG-15* (AeroFax, 2001), pp. 121 and 114–116, and Arsenyev and Krylov, *Истребитель МиГ-15* (Armada no. 10, 1999), p. 78. The page lists normal take-off weight 4,960 kg but does not tie it to the ceiling. It says state tests of the "SD" began at ГК НИИ ВВС on 14 September 1949 and the act was approved by decree of 10 June 1950. Secondary.
- 15,500 m. Same figure in the data table of C9 (MiG-15bis page), with empty weight 3,582 kg, normal take-off 4,960 kg, maximum 6,105 kg. Secondary.
- 15,550 m (orig.) = 51,016 ft as printed. "Service ceiling (for take-off weight 5044 kg)", C11. No source stated. Secondary.

Recommended: 15,500 m, low confidence by the rule used here (secondary only). Three independent secondary sources agree within 50 m, and one traces the figure to a named state-test aircraft.

Against the two existing sets: the sim's 15,500 m matches. The tool's 15,000 m is a round number 500 m low; no source for it was found.

### La-11

Figures found:

- 10,250 m (orig.) = 33,629 ft. Practical ceiling, take-off weight 3,730 kg, year 1947. Table on C8 (La-11 page). The page names Shavrov, *История конструкций самолетов в СССР, 1938–1950 гг.*, and a supplement to M-Hobby no. 11 as sources. Its text says the first aircraft went to state tests at ГК НИИ ВВС on 19 June, and that range and practical ceiling were the only requirements the aircraft met. It gives no measured ceiling from those tests. Secondary.
- 10,250 m. Same figure on C9 (La-11 page; weights 2,770 / 3,730 / 3,996 kg), Russian Wikipedia (https://ru.wikipedia.org/wiki/Ла-11) and English Wikipedia (https://en.wikipedia.org/wiki/Lavochkin_La-11). Neither Wikipedia page cites a source for the figure.

Recommended: 10,250 m, low confidence. All sources read agree.

Against the two existing sets: the sim's 10,300 m is the same figure rounded up. The tool's 10,150 m is 100 m low with no source found.

### Yak-9P

Figures found:

- 10,500 m (orig.) = 34,449 ft. Practical ceiling, take-off weight 3,550 kg, empty 2,708 kg, year 1947. Table on C8 (Yak-9P page). The page names Leipnik, *Як-9: рядовые небес*; Kosminkov in *АиВ* no. 2, 2005; and *Война в воздухе* no. 34 as sources. No test type is stated for the table. Secondary.
- The same table has a 1944 column also headed "Як-9П" with 8,600 m at 3,356 kg. That column is the wartime cannon variant tested in 1943, not the post-war all-metal Yak-9P.
- 10,650 m (orig.) = 34,941 ft. Yak-9U, 3,204 kg. Russian Wikipedia, https://ru.wikipedia.org/wiki/Як-9 , citing Medved' and Shavrov 1988. It gives no Yak-9P ceiling.

Recommended: 10,500 m, low confidence. One secondary source only.

Against the two existing sets: the tool's 10,600 m is 100 m high. The sim's 11,000 m is 500 m high and matches nothing found. The tool is closer.

### Il-10

Figures found:

- 7,250 m (orig.) = 23,786 ft. Practical ceiling, flight weight 6,300 kg, empty 4,650 kg. Data table on C9 (Il-10 page). Secondary.
- 7,270 m (orig.) = 23,852 ft. Given in the body text of the same page at 6,300 kg, in a comparison with the Il-2 in the passage on the 1944 state tests. The page text arrived garbled, so the context of this figure is not certain.
- 7,250 m. Russian Wikipedia, https://ru.wikipedia.org/wiki/Ил-10 . The section names Yakubovich, *Все самолеты Ильюшина* (Yauza/Eksmo, 2013), p. 214, as its main source. No weight given. Secondary.
- 7,000 m (orig.) = 22,966 ft. Il-10U trainer, normal take-off 5,680 kg, state tests of a production aircraft. C8 (Il-10 page). Not the combat version.

Recommended: 7,250 m, low confidence. Two secondary sources agree and a third figure is within 20 m.

Against the two existing sets: the tool's 6,950 m is 300 m low. The sim's 6,000 m is 1,250 m low. The tool is closer. No definitional difference found explains the sim's figure.

### Tu-2 (Tu-2S, ASh-82FN)

Figures found:

- 9,500 m (orig.) = 31,168 ft. Practical ceiling, Tu-2 with ASh-82FN, year 1943, take-off weight 10,360 kg normal / 11,360 kg maximum. Table on C8 (Tu-2 page), which names Shavrov among its sources. The page text says the tests of this aircraft ran from 4 September to 16 December 1943. Secondary.
- 9,000 m (orig.) = 29,528 ft. Practical ceiling, production Tu-2 of 1942 with two M-82 engines (not the FN), normal take-off 10,538 kg. Russian Wikipedia, https://ru.wikipedia.org/wiki/Ту-2 . It credits the НИИ ВВС state-test act for production aircraft no. 100308 from Plant 166. The act was not read. This is the earlier variant.
- 10,600 m (orig.) = 34,777 ft. Post-war variant with ASh-83 engines, state tests 22 August to 21 November 1946. C8. Not the Tu-2S.
- C7, the 1946 technical description (Book 3), was found. The first 100,000 characters cover structure only and give no ceiling. The rest was not read.

Recommended: 9,500 m, low confidence. One secondary source for the ASh-82FN variant.

Against the two existing sets: the tool's 10,500 m is 1,000 m high and close to the ASh-83 variant's 10,600 m. The sim's 8,000 m is 1,500 m low. The tool is closer in metres, but neither matches. A variant mix-up may explain the tool's figure. Nothing found explains the sim's.

### Li-2T

Figures found:

- 5,600 m (orig.) = 18,373 ft. "Ceiling", table headed Li-2 (passenger), two M-62IR. Russian Wikipedia, https://ru.wikipedia.org/wiki/Ли-2 . The table lists 7,675 kg and 10,700 kg as weights without tying either to the ceiling. The page's literature list includes the Oborongiz technical descriptions of 1947 (ed. Soloukhin) and 1951 (ed. Sloushch). Two readings of the page disagreed on whether the figure is footnoted to them, so treat it as unsourced. Secondary.
- 5,750 m (orig.) = 18,865 ft. Practical ceiling, PS-84 with M-62IR, 21 passengers. Table on C8 (Li-2 page). The page text adds that the defensive armament of the military version lowered the practical ceiling by about half a kilometre. Secondary.
- 9,100 m and 8,870 m in the same table are for versions with ASh-82FN engines. They do not apply to the Li-2T.

Recommended: 5,600 m, low confidence. No figure specific to the Li-2T transport was found. The armed transport is likely somewhat below the passenger figure, per the airpages.ru remark.

Against the two existing sets: the sim's 5,500 m is 100 m low and consistent with the armament remark. The tool's 6,800 m is 1,200 m high and matches nothing found. The sim is closer.

## 4. Not found / open

1. F-80C, F-84E, F-86A: no primary source read. No SAC chart for these three was found: archive.org has none by title, and the generalstaff.org index (read in a browser on 2026-10-12) has no page for the F-80, F-84, F-86, F-51 or C-47. The charts themselves, or the flight handbooks T.O. 1F-80C-1, 1F-84E-1 and 1F-86A-1, are the documents to find.
2. F-80C: two figures 4,050 ft apart (Knaack 42,750 ft, others 46,800 ft). The cause is not established.
3. F-84E: Knaack 45,000 ft against the museum sheet's 43,240 ft. The cause is not established.
4. F-86A: Knaack gives no figure. The "combat ceiling 49,000 ft" label on the museum sheet conflicts with the 48,000 ft service ceiling.
5. F-51D: no SAC or Characteristics Summary found. The 1954 Flight Handbook was read in part and states no ceiling.
6. B-29: no service ceiling at combat weight found. The full SAC chart was not readable.
7. C-47B: no primary figure with the high blower working. The 1963 manual covers only the blower-inoperative case, and only one sample point was legible.
8. MiG-15bis, La-11, Yak-9P, Il-10, Tu-2S, Li-2T: no technical description, pilot's instruction or НИИ ВВС act was read. All six rest on web pages that name books as their sources.
9. Tu-2: characters 100,000–261,309 of the 1946 technical description (C7) were not read. Book 3 covers structure, so a ceiling figure there is unlikely.
10. Li-2T: no figure for the transport version by name. The 1947 and 1951 technical descriptions are the documents to find.
11. Tool values with no source found: F-51D 11,278 m (37,000 ft), F-86A 15,240 m (50,000 ft), C-47B 6,096 m (20,000 ft), Li-2T 6,800 m. Sim values with no source found: Yak-9P 11,000 m, Il-10 6,000 m, Tu-2 8,000 m, C-47B 8,600 m.

## 5. Is the gap between the tool and the game a correction?

No. A correction (indicated against true height, a standard-day adjustment, feet against metres) would move every aircraft the same way. The differences go both ways and vary from 0 to 1,950 m.

| Aircraft | Recommended (m) | Tool before 2026-10-13, difference | Sim IL-2 Korea, difference |
|---|---|---|---|
| F-80C-10 | 13030 | +686 | −130 |
| F-84E | 13716 | −1524 | −16 |
| F-86A-5 | 14630 | +610 | +270 |
| F-51D | 12680 | −1402 | +20 |
| B-29 | 10668 | 0 | +32 |
| C-47B | 8047 | −1951 | +553 |
| MiG-15bis | 15500 | −500 | 0 |
| La-11 | 10250 | −100 | +50 |
| Yak-9P | 10500 | +100 | +500 |
| Il-10 | 7250 | −300 | −1250 |
| Tu-2S | 9500 | +1000 | −1500 |
| Li-2T | 5600 | +1200 | −100 |

What the pattern shows:

- **The tool's old NATO figures were round numbers in feet** (45,000, 40,000, 50,000, 37,000, 35,000, 20,000 ft). Two of them belong to a different variant: 45,000 ft is the F-80A/B, and 40,000 ft sits by the F-84G. The Tu-2 figure sits by the ASh-83 variant. They read as estimates, not as figures taken from a document.
- **The game's handbook is within 130 m of the recommended figure for 7 of the 12** (F-80C, F-84E, F-51D, B-29, MiG-15bis, La-11, Li-2T). For those it has plainly used the same published figures, rounded to 100 m.
- **Five game figures match no source found** (F-86A +270 m, C-47B +553 m, Yak-9P +500 m, Il-10 −1,250 m, Tu-2 −1,500 m). The game may compute these from its flight model at a loaded weight; that is a guess and nothing read supports or refutes it.

## 6. How the Knaack tables were read

The scan of C5 on archive.org carries a text layer with the position of every word (`DTIC_ADA057002_djvu.xml`). The three Technical Data tables (printed pp. 11, 50, 80; leaves 22, 61, 91) were read from that layer on 2026-10-12, grouping words into rows by height and into columns by their left edge. Each value was assigned to the heading whose left edge it shares. The page images were not viewed.
