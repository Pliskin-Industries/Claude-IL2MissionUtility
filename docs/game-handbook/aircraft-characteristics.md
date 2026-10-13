# Aircraft figures from the game's own handbook

Harvested 2026-10-12 from Korea. IL-2 Series, version `261002` (launcher 1.004), by reading the screen.

- **Where in the game:** fly any mission, press Esc, **Handbook**, tab **Characteristics**. **Compare** puts a second aircraft beside the one you are flying; every row below was read from that comparison against the F-80C-10.
- **How reliable:** each number was read once from a screen capture and typed by hand. A chart reading is marked "about". Before a figure is used in code, read it again in the game.
- **Not harvested:** the Cockpit Layout tab (instrument lists), the Flight Manual tab for aircraft other than the F-80C, the Historical Data tab (prose), and the Museum pages for ground vehicles, armor, artillery, railroad, equipment and ships. The armament and engine tables were read for the F-80C only, with a few rows for others.

## What the speed rows mean

- **Cruising speed** carries the tooltip "Instrumental, indicative": it is an **indicated** airspeed, the economical cruise, and far below what the aircraft does at high power.
- **Maximum speed** is also indicated ("Instrumentation near the ground"); the F-80C flight manual lists it as a limit.
- **Maximum continuous** and **maximum short-term** are the top of the speed charts (nominal and combat engine mode), in **true** speed. Checked in flight: the F-80C chart shows about 890 km/h in combat mode at 3000 m, and a timed 24 km leg at 100 % throttle and 2990 m gave 877 km/h (`handoff/P14-probe-results.md`, flight 1N).
- A dash means the handbook shows a dash. "0 to 0" is what the handbook prints for the Mach number of some piston aircraft.

## Speeds and ceiling

Speeds in km/h, ceiling in m, climb in m/s.

| Aircraft | Script | Cruising (indicated) | Max continuous | Max short-term | Maximum (indicated) | Max Mach | Practical ceiling | Max climb |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| F-80C-10 | `f80c10` | 348 | 837 | 890 | 933 | 0.8 | 12900 | 27.5 |
| F-84E | `f84e` | 540 | 975 | 990 | 1010 | 0.82 | 13700 | 37.6 |
| F-86A-5 | `f86a5` | 640 | 1038 | 1080 | 1164 | 0.95 | 14900 | 46.4 |
| F-51D | `f51d` | 430 | 674 | 709 | 812 | 0.78 | 12700 | 17.4 |
| B-29 | `b29` | 413 | 586 | 629 | 640 | 0.77 | 10700 | 7.3 |
| C-47B | `c47b` | 235 | 376 | – | 450 | 0 to 0 | 8600 | 6.3 |
| MiG-15bis | `mig15bis` | 520 | 1038 | 1078 | 1125 | 1 | 15500 | 50 |
| La-11 | `la11` | 460 | 666 | 684 | 760 | 0.77 | 10300 | 18.5 |
| Yak-9P | `yak9p` | 380 | 668 | 675 | 720 | 0.77 | 11000 | 19.4 |
| IL-10 | `il10` | 330 | 550 | – | 590 | 0 to 0 | 6000 | 10.3 |
| Tu-2 | `tu2` | 330 | 557 | – | 680 | 0 to 0 | 8000 | 10.4 |
| Li-2T | `li2t` | 230 | 319 | – | 450 | 0 to 0 | 5500 | 4.4 |

## Low-speed figures, range and endurance

Speeds in km/h, distances in m, range in km, duration in hours.

| Aircraft | Landing | Stall | Stall, gear and flaps | Take-off run | Take-off run, max weight | Range | Range, drop tanks | Duration | Duration, drop tanks |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| F-80C-10 | 177 | 165–230 | 172–210 | 731.52 | – | 844 | 1586 | 1.8 | 3.5 |
| F-84E | 201–225 | 193–267 | 185–255 | 985 | not read | 677 | 1359 | 0.8 | 1.8 |
| F-86A-5 | 204 | 193–218 | 179–205 | 640.08 | 975.36 | 632 | 1183 | 0.6 | 1.5 |
| F-51D | 150 | 159–196 | 147–181 | 426.72 | 701.04 | 2584 | 4238 | 7.9 | 14 |
| B-29 | 346 (as printed) | 206–258 | 191–239 | 1043.94 | not read | 3164 | – | 10.2 | – |
| C-47B | 108 | 121–143 | 116–126 | 430 | not read | 2477 | – | 10.1 | – |
| MiG-15bis | 170 | 213 | 185 | 475 | 570 | 714 | 924 | 1.3 | 1.9 |
| La-11 | 165 | 170–186 | 154–170 | 550 | not read | 2343 | – | 6.6 | – |
| Yak-9P | 175 | 168–182 | 152–168 | 380 | not read | 1699 | – | 5.3 | – |
| IL-10 | 150–155 | 169–187 | 150–156 | 560 | not read | 1014 | – | 3.4 | – |
| Tu-2 | 150–170 | 200–217 | 155–190 | 530 | not read | 1847 | – | 5.5 | – |
| Li-2T | 105 | 116–130 | 110–115 | 460 | not read | 2594 | – | 11.7 | – |

## Size and weight

Lengths in m, area in m², wing loading in kg (per m², as the handbook labels it), weights in kg.

| Aircraft | Crew | Length | Wingspan | Wing area | Wing loading | Empty | Fuel | Normal take-off | Maximum take-off |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| F-80C-10 | 1 | 10.52 | 11.85 | 22.1 | 265.1 | 4013.5 | 1302.3 | 5850.7 | 8527.5 |
| F-84E | 1 | 11.73 | 11.1 | 24.1 | 272.4 | 4745 | 2689 | 8093 | 9990 |
| F-86A-5 | 1 | 11.43 | 11.31 | 26.8 | 238.7 | 4340 | 1317 | 6385 | 7815 |
| F-51D | 1 | 9.84 | 11.29 | 22.3 | 193.3 | 3325 | 732 | 4347 | 5650 |
| B-29 | 11 | 30.18 | 43.04 | 159.8 | 397.4 | 32431.9 | 14862.6 | 54431.2 | 63503 |
| C-47B | 3 | 19.65 | 28.96 | 91.7 | 129.8 | 8326 | 2188 | 11905 | 13000 |
| MiG-15bis | 1 | 10.04 | 10.08 | 20.6 | 244.9 | 3476 | 1173 | 5044 | 6106 |
| La-11 | 1 | 8.63 | 9.8 | 17.7 | 211 | 2936.5 | 808.5 | 3739 | 4072 |
| Yak-9P | 1 | 8.55 | 9.77 | 17.2 | 196 | 2771 | 516 | 3374 | 3550 |
| IL-10 | 2 | 11.12 | 13.4 | 30 | 195.9 | 4650 | 537 | 6300 | 6500 |
| Tu-2 | 4 | 13.8 | 18.86 | 48.8 | 227.5 | 8600 | 2093 | 11100 | 14100 |
| Li-2T | 4 | 19.6 | 28.8 | 91.7 | 116.7 | 7695 | 2240 | 10700 | 10700 |

## General information

| Aircraft | Country | Developer | First flight | Production | In service | Built |
|---|---|---|---|---|---|---:|
| F-80C-10 | USA | Lockheed | 1944-01-08 | 1945–1950 | 1945–1958 | 1718 |
| F-84E | USA | Republic Aviation Corporation | 1949-05-18 | 1949–1953 | 1949–1959 | 4457 |
| F-86A-5 | USA | North American | 1948-05-20 | 1947–1956 | 1948–1984 | 8745 |
| F-51D | USA | North American | 1940-10-26 | 1941–1951 | 1942–1984 | 16974 |
| B-29 | USA | Boeing | 1942-09-15 | 1943–1946 | 1943–1955 | 3965 |
| C-47B | USA | Douglas Aircraft | 1941-12-23 | 1941–1945 | 1941–1969 | over 10,000, all variants |
| MiG-15bis | USSR | Design Bureau 155 | 1947-12-30 | 1949–1959 | 1949–1970 | 8354 |
| La-11 | USSR | shown as "USSR" | 1947-05-03 | 1947–1951 | 1948–1955 | 1232 |
| Yak-9P | USSR | shown as "USSR" | 1946-08-09 | 1946–1948 | 1950–1960 | 801 |
| IL-10 | USSR | shown as "USSR" | 1944-04-18 | 1944–1949 | 1945–1956 (USSR), to 1962 (Czechoslovakia), to 1972 (China) | 4390 |
| Tu-2 | USSR | TsKB-29 | 1941-01-29 | 1942, 1944–1952 | 1942–1954 (USSR), to 1982 (PRC) | 2540 |
| Li-2T | USSR | Douglas Aircraft, USA | 1939-10-17 | 1939–1953 | 1939–1976 | 5207 |

These dates are the handbook's. Some differ from the historical reference in `docs/historical-reference/` (for example the F-84E and F-86A production years); the handbook is the record of what the game says, not a historical source.

## F-80C-10 in full

**Speed chart (read off, about):** nominal mode 766 km/h at sea level, 812 at 3000 m, peak 837 at 7000 m. Combat mode 860 at sea level, peak 890 at 3000 m, 796 at 11,666 m.

**Other flight figures:** destructive overload 11.5 G; turn time 18.4 s at ground level and 29.9 s at 5000 m; aerodynamic quality 14.3.

**Engine:** J33-A-35 turbojet, fuel JP-3. Thrust 1979 maximum and 1586 nominal (the handbook prints the unit as kg/s). Fuel use: 2575 L/h maximum, 2265 nominal, 1010 cruising.

| Engine mode | Time limit | Exhaust gas temperature | RPM | Fuel, L/h |
|---|---|---:|---:|---:|
| Take-off, with water injection | 5 min | 715 | 11750 | 2575 |
| Combat | 30 min | 715 | 11750 | 2576 |
| Nominal | unlimited | 655 | 11250 | 2265 |

**Limits (flight manual):** 933 km/h indicated; 362 with gear down; 322 with full flaps; Mach 0.8; 7.3 G without external loads, 5.3 G with; landing light not above 402 km/h. Take-off with 70 % flaps at 201 to 241 km/h; full-load take-off run up to 1463 m. Landing: glide 241, approach 193, touch down about 177 km/h.

**Armament:** six 12.7 mm M3 guns, 1800 rounds, 1100 rounds per minute. Bombs: M64A1, M65A1, M26A2 cluster, M29 cluster, 416 L napalm, M88, M57A1, four of each. Eight HVAR rockets. Drop tanks: two 625 L or two 1003 L. Maximum load 2243 kg.

**Armor:** glass 1½ in; backrest ¼ in; headrest ⅜ in; ¼ in sheet ahead of the windscreen; ¼ in partition ahead of the cockpit.

## Other rows read

| Aircraft | Row | Value |
|---|---|---|
| MiG-15bis | Fuel use, L/h | 3133 maximum, 2680 nominal, 2380 cruising |
| MiG-15bis | Guns | one 37 mm N-37D (40 rounds), two 23 mm NS-23KM (160 rounds) |
| MiG-15bis | External | two FAB-100 bombs; two 250 L drop tanks; load 364.7 kg |
| MiG-15bis | Armor | glass 64 mm; plate 10 mm; headrest 10 mm |
| C-47B | Engine | PW-1830-90C, 29.99 L; 1200 hp maximum, 1050 nominal |
| C-47B | Flight | 5 G destructive; turn 25 s at ground, 33 s at 3000 m; quality 13.3 |
| B-29 | Engine | R-3350-79, 54.9 L; 2200 hp maximum, 2000 nominal; fuel 4542.5 L/h emergency, 3543.1 nominal |
| B-29 | Flight | 4.5 G destructive; quality 14.2 |
| Tu-2 | Engine | ASH-82FN, 41.2 L; 1850 hp maximum, 1630 nominal |
| Tu-2 | Flight | 9 G destructive; turn 33 s at ground, 43 s at 3000 m; quality 12 |
| Li-2T | Engine | ASH-62IR, 29.87 L; 1000 hp maximum, 840 nominal |
| Li-2T | Flight | 5 G destructive; turn 25 s at ground, 46 s at 3000 m; quality 12.3 |
| IL-10 | Weight | armor 914 kg; the 6500 kg maximum is "full fuel, 600 kg bombs and missiles (overload)" |

## Compared with `src/model_spec.rs`

For ceilings, the comparison against operating documents is in `docs/historical-reference/aircraft-service-ceilings.md` (its "Sim IL-2 Korea" column is this page's practical ceiling).

The app's table gives each aircraft one cruise speed and one ceiling. Since 2026-10-13 the ceilings are documented figures with their sources noted in `src/model_spec.rs`; the cruise speeds have no source. The handbook's ceiling is the same kind of figure. Its cruising speed is not: it is an indicated economy speed, where the app's figure is used as a transit speed.

| Aircraft | App cruise | Handbook cruising | Handbook max continuous | App ceiling | Handbook ceiling |
|---|---:|---:|---:|---:|---:|
| F-80C-10 | 732 | 348 | 837 | 13030 | 12900 |
| F-84E | 770 | 540 | 975 | 13716 | 13700 |
| F-86A-5 | 800 | 640 | 1038 | 14630 | 14900 |
| F-51D | 583 | 430 | 674 | 12680 | 12700 |
| B-29 | 350 | 413 | 586 | 10668 | 10700 |
| C-47B | 257 | 235 | 376 | 8047 | 8600 |
| MiG-15bis | 850 | 520 | 1038 | 15500 | 15500 |
| La-11 | 500 | 460 | 666 | 10250 | 10300 |
| Yak-9P | 554 | 380 | 668 | 10500 | 11000 |
| IL-10 | 436 | 330 | 550 | 7250 | 6000 |
| Tu-2 | 465 | 330 | 557 | 9500 | 8000 |
| Li-2T | 240 | 230 | 319 | 5600 | 5500 |

One measured point: AI F-80Cs flying the built-in template (waypoint `Speed = 660`, which is 90 % of the app's 732) crossed a 20 km circle at 733 km/h over the ground (flight 1N).
