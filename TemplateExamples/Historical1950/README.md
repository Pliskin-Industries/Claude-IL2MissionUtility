# Early-war 1950 air templates

Six Template Builder groups for June–December 1950, built from the sourced reference [`docs/historical-reference/korea-1950-53-unit-reference.md`](../../docs/historical-reference/korea-1950-53-unit-reference.md) (§2.2 and §7). Citations like `[S9 p.53]` point into that reference; its evidence file quotes each source line.

They were generated with the app's own `template::generate_template`, so each one has the normal Template Builder logic:
- **Spawn:** units come up when an enemy player enters **Zone IN** (16 km).
- **Orders:** the lead runs its orders in sequence.
- **Cleanup:** units RTB and are deleted after **Zone Out** (35 km) or when their orders complete.

Each file was reloaded with `template::load_template`: seats and orders round-trip with no warnings, and `bombers::inspect_plan` accepts every file, so any of them can be a plan in **Exclusive Activation**.

**To use a template:**
1. Import the `.Group` in the mission editor (or load it in Template Builder).
2. Move the group onto the map.
3. Put **WP 1** on the target area. The ground AttackArea is flown at WP 1.
4. Adjust skill (all are Average, `AILevel 2`) and the mission time.

| File | Aircraft | Orders (lead) | Sourced | My choices |
|---|---|---|---|---|
| `1950_US_F80_AirAlert_4ship` | 4 × F-80C, guns only, 2 × 165-gal tanks (payload 2) | Formation Wedge → WP 1 at 3,050 m → AttackArea **air**, 5 km, **18 min** → AttackArea ground targets, 1.5 km, 2 min (strafing) → Mission Complete | Flights of 4 [S9 p.49]; orbit at 3,050 m [10,000 ft] for 15–20 min, then 1–2 strafing passes [S9 p.53]; guns only in July, 165-gal tanks = about 15 min over target [S9 p.81, p.109] | 5 km orbit radius; 2 min for "1–2 passes" |
| `1950_US_F80_HVAR_Strike_4ship` | 4 × F-80C, 8 × HVAR + 2 × 165-gal (payload 4) | Formation Wedge → WP 1 at 3,050 m → AttackArea ground targets, 2 km, 10 min → Mission Complete | HVAR attack on armor from the 4 o'clock position, 30° dive, fire at about 460 m [1,500 ft], all four in one salvo [S9 p.111]. Up to 16 HVAR historically [S9 p.81]; the game's maximum is 8 | Approach at 3,050 m. The AI flies its own dive; the 30° profile can't be set |
| `1950_US_F51_CloseSupport_4ship` | 4 × F-51D, 2 × 500 lb (payload 1; **6** = napalm) | Formation Wedge → WP 1 at 1,500 m → AttackArea ground targets, 1.5 km, 15 min → Mission Complete | Bomb run, then strafing passes [S9 p.156]; flights of 4 [S9 p.189]; napalm variant [S9 p.184] | **1,500 m approach is unsourced** (the ridges crossed were 1,520–1,830 m [S9 p.112]) |
| `1950_US_F51_ArmedRecon_2plus2` | 2 + 2 × F-51D, 2 × 110-gal + 4 × HVAR (payload 25) | Low pair (Red): Pairs → WP 1 at 900 m → AttackArea ground targets, 3 km, 20 min → Mission Complete. High pair (Blue): Pairs → **Cover** the low pair | Lower element searches for traffic, upper element watches for MiGs (Nov–Dec 1950) [S9 p.268]; drop tanks on long missions [S9 p.255] | **900 m and 2,400 m element altitudes are unsourced**; payload 25 is my choice |
| `1950_DPRK_Yak_AirfieldRaid_2plus2` | 2 + 2 × Yak-9P (standing in for the Yak-3/7B/9), guns | Each pair: Pairs → WP 1 at 3,050 m → AttackArea ground targets, 1.5 km, 5 min → Mission Complete | Raids of 2–6 aircraft in pairs, threes or fours [S9 p.121]; 3,050 m [10,000 ft] on the way to Kimpo [S9 p.34]; strafed aircraft and airfields [S9 p.50] | 5 min over the target |
| `1950_DPRK_Il10_KimpoAttack_2x4` | 2 × 4 × Il-10, 128 × AO-2.5 frag (payload 25) | Each four: Formation Wedge → WP 1 at 1,500 m → AttackArea ground targets, 1.5 km, 10 min → Mission Complete | 8 Il-10s attacked transport aircraft at Kimpo on 27 Jun 1950 [S9 p.35] | **Altitude and load are unsourced**. Historically they broke off after losing 4 [S9 p.35]; that is **not wired**, and damaged aircraft rely on AI RTB (`AiRTBDecision 1`) |

**Zone triggers.** US templates watch Eastern players (`PlaneCoalitions = [1]`); North Korean templates watch Western players (`[2]`).

**Not in this set.**
- **No La-9 or La-11.** Neither flew in Korea in 1950. The La-9 first appears on 30 Nov 1951, escorting Tu-2s [S9 p.437], and the La-11 in Apr 1953 with the night hecklers [S9 p.686].
- **No F-80 relief rotation.** Historically a new F-80 flight launched every 20 min [S9 p.101]. Template Builder can't respawn a flight with linked wingmen (`generate_template` forces Activate), so build that rotation in **Fighter Pack** instead:
  - F-80C only, max 4 per flight;
  - altitude min/max around 3,000–3,100 m;
  - Reinforcement / Cooldown near 1,200 s. Check how these two timers behave in the app before relying on them.
- **No trigger linking the Yak raid to the F-80 patrol.** The Yak raids were timed for when the F-80s ran short of fuel [S9 p.121]. That link has to be wired in the editor (for example, a timer started by the F-80 group's Mission Complete).
