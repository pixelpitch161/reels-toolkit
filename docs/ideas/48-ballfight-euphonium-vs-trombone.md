# 48. BALLFIGHT: Euphonium vs Trombone

Board row: `docs/IDEAS.md` (#48). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 48. BALLFIGHT: Euphonium vs Trombone — APPROVED
**Direction (user, Oct 28):** "draft Euphonium vs Trombone"; approved with mood A (dark New Orleans second line). A
crossover: EUPHONIUM as built in #38, TROMBONE as built in #34 / #41 (kit module from #41). All BALLFIGHT series rules
(VS title card without an episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight,
61.0 s, loop point = file boundary), the crossover rules (fighters exactly as in `fighters.py`; the only per-episode
lever is a per-fighter damage handicap, 40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** valves vs slide: the same notes, the same band parts, the same register (half of all euphonium players get
handed a trombone at some point): identical range, opposite mechanics. Post caption: "Same notes, different weapon.
Valves or slide? Comment the next match-up 👇"

| | EUPHONIUM (amber, left), from #38 | TROMBONE (mint, right), from #34 / #41 |
|---|---|---|
| Signature | BELL MORTAR (shells arc high, land on the predicted spot; shadow circles) | SLIDE LANCE (thrust on every beat; swats what it passes through) |
| Grows per landed hit | +shells per salvo | Lance reach |
| LV2 | MARCHING ORDERS (shells every beat) | SAD TROMBONE (shockwave + 1 s slow) |
| LV3 | IDENTITY CRISIS (copies the enemy's last ability) | POWER SLIDE (thrusts on 8ths) |
| Below 25 HP | NOT A TUBA (full barrage) | PEDAL TONE (a giant bell drops and stomps) |

- **Likely shape (only if the honest fight gives it):** range vs reach; the comedy beat to hope for: IDENTITY CRISIS
  copies SAD TROMBONE and fires it back in amber ("wah wah wah waaah": for a moment the euphonium IS a trombone). The
  lance may swat shells mid-air if the engine's swat rule covers them; if that interaction is undefined, add it as an
  opt-in rule (own commit) and tell the user.
- **Sides:** EUPHONIUM left (swapped from #38 for this episode only), TROMBONE right (its usual side). **Colours:** amber
  vs mint.
- **Music, mood A (dark New Orleans second line):** new kit mood `kit/moods/second_line.py`: a minor brass-band street
  groove (snare with press rolls, bass drum with cymbal on top, sousaphone-style bass kept low and simple, no horns of
  its own). Same register for both fighters, so split the jobs: the **euphonium plays the warm melody** (its shells =
  melody notes), the **trombone the punchy riffs and smears** around it (its thrusts = riff stabs); leader fuller and
  louder, loser thinner and dirtier; the winner's solo over the second line. `loop_bpm(122)` = 121.97 BPM (31 bars;
  bar = 1.968 s, FIGHT! at 3.94 s). Consistent loudness from frame 0. (Not chosen: B low-brass mambo; C dark disco.)
- **VS title card:** EUPHONIUM / VS / TROMBONE: a euphonium blast, the thunder crack, a trombone smear.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.94 s · unlocks 8–30 s · desperation 32–44 s ·
  K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** TROMBONE is in the kit. EUPHONIUM: if the porting session hasn't ported it yet, port it here from #38
  (`kit/fighters/euphonium.py`, checked against #38 with the hash and a few frames) and note it in
  `docs/log/ballfight_studio.md` so it isn't done twice. Engine changes additive; every earlier episode replays
  identically. Kit `check` must pass.
- **Effort:** ~30–45 min. Folder: `projects/ballfight_euphonium_vs_trombone/`.

## Board history

- Approved Oct 28 (ideas chat).
