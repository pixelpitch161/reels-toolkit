# 46. BALLFIGHT: Trumpet vs Clarinet

Board row: `docs/IDEAS.md` (#46). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 46. BALLFIGHT: Trumpet vs Clarinet — APPROVED
**Direction (user, Oct 28):** "draft trumpet vs clarinet"; approved with mood A (dark hot jazz). A crossover: TRUMPET as
built in #29 v3, CLARINET as built in #37. All BALLFIGHT series rules (VS title card without an episode tag, 3·2·1,
ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary), the
crossover rules (fighters exactly as in `fighters.py`; the only per-episode lever is a per-fighter damage handicap,
40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** who leads the jazz band? In New Orleans jazz the trumpet blasts the melody and the clarinet weaves around
it; later the clarinet had its swing-era moment as the bandleader's instrument. Post caption: "Who really leads the
band? 🎺 Comment the next match-up 👇"

| | TRUMPET (yellow, left), from #29 v3 | CLARINET (lavender, right), from #37 |
|---|---|---|
| Signature | VALVE COMBO (three piston punches 1-2-3, steam) | TWELFTH JUMP (teleports beside the enemy on the beat; the zip line cuts) |
| Grows per landed hit | Piston reach | Jump distance |
| LV2 | FANFARE (shockwave) | SQUEAK (0.5 s stun) |
| LV3 | DOUBLE TONGUE (combos twice as fast) | CLARION (jumps twice as often) |
| Below 25 HP | HIGH C (giant golden beam) | RHAPSODY (five rising jump-cuts) |

- **Likely shape (only if the honest fight gives it):** brawler vs teleporter: the pistons punish the clarinet whenever
  it lands close, but it never stays; SQUEAK can catch the trumpet mid-combo; finish: HIGH C's beam vs RHAPSODY.
- **Sides:** TRUMPET left (its usual side), CLARINET right (swapped for this episode only). **Colours:** yellow vs lavender.
- **Music, mood A (dark hot jazz):** new kit mood `kit/moods/hot_jazz.py`: a fast minor two-beat (tuba-style bass on 1
  and 3, piano-less rhythm guitar chop on 2 and 4, snare press rolls, brushed cymbals); no horns or clarinet of its own.
  The **trumpet blasts the melody**, the **clarinet weaves the counter-line** above and around it (a jazz front line);
  leader fuller and louder, loser thinner and dirtier; the winner's solo over the two-beat. `loop_bpm(140)` = 141.64 BPM
  (36 bars; bar = 1.694 s, FIGHT! at 3.39 s). Consistent loudness from frame 0. (Not chosen: B klezmer; C dark disco.)
- **VS title card:** TRUMPET / VS / CLARINET: a trumpet blast, the thunder crack, a clarinet gliss.
- **Beat sheet:** the series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.39 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** CLARINET is already in the kit. TRUMPET: if the porting session hasn't ported it yet, port it here from #29 v3
  (`kit/fighters/trumpet.py`, checked against #29 with the hash and a few frames; #29 has the retired SAX, so verify the
  trumpet from that episode alone), and tell the porting session's log so it isn't done twice. Engine changes additive;
  every earlier episode replays identically. Kit `check` must pass.
- **Effort:** ~30–45 min (less if TRUMPET is already ported). Folder: `projects/ballfight_trumpet_vs_clarinet/`.

## Board history

- Approved Oct 28 (ideas chat).
- In production Oct 28 (session_01CzTfqa). TRUMPET ported into the kit from #29 v3 (`kit/fighters/trumpet.py`; check
  `examples/trumpet_vs_sax`: replay hash identical, 12 frames pixel-identical in a matched harness, trumpet bus to
  1e-16; noted in `docs/log/ballfight_studio.md`). New kit option `FIGHT_BAR` (for #29's check config only).
- New mood `kit/moods/hot_jazz.py` (D minor two-beat, Dm Dm Gm Gm Bb7 A7 Dm A7, 141.64 BPM): the trumpet's punches
  play the melody (LINES, 8ths), the clarinet's jumps the counter-line (LINES, beats). Handicap DMG_SIDE (0.34, 0.41):
  300 seeds 43 % / 57 %, median K.O. 49.6 s. Seed 8410 (8 fits in 9,267 runs): 3 lead changes, RHAPSODY 41.1 then
  HIGH C 41.9, CLARINET wins at 49.99 s with 3.4 HP. Sheet sent first; all checks pass; one full render (273 s +
  164 s encode); verify PASS. Delivered as review: `ballfight_trumpet_vs_clarinet.mp4`.
