# 43. BALLFIGHT: Kazoo vs Harmonica

Board row: `docs/IDEAS.md` (#43). Series rules: `docs/ballfight/RULES.md`.

## 43. BALLFIGHT: Kazoo vs Harmonica — APPROVED
**Direction (user, Oct 27):** "Draft Kazoo vs Harmonica. From now on, don't have the Ballfight # in the video";
approved with mood A (swamp blues stomp). All BALLFIGHT series rules (VS title card **without an episode tag**, 3·2·1,
ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary) plus the
PLAYBOOK's consistent loudness from frame 0 (`reelkit.levels` must PASS).
**Rivalry:** the pocket instruments: the two anyone can "play" and everyone has an opinion about; a joke match-up
with a real fight. Post caption: "Which one belongs in the junk drawer? 😂 Comment the next match-up 👇"

| | KAZOO (red, left) | HARMONICA (sky blue, right) |
|---|---|---|
| Style | The only multi-body fighter | The only fighter that rides rails |
| Signature | **KAZOO ARMY**: kazoos are cheap and come in packs: every bar it splits into mini-kazoos that zip at the enemy buzzing, then regroup. Only the main ball can be hurt; the minis are attackers the enemy can swat away (destroyed, no HP lost) (new kind `army`) | **FREIGHT TRAIN**: the classic harmonica train: a glowing track lays itself across the ring toward the enemy (telegraphed, ~1 beat), then the harmonica barrels down it like a locomotive, chugging, whistle on impact (new kind `rails`) |
| Grows per landed hit | +1 mini-kazoo (max 5) | Train speed + track length |
| LV2 | BUZZ: a buzzing shockwave; the enemy shakes, its aim wobbling for 1 s (new `jitter`) | BLUES BEND: the tracks bend, curving to follow the enemy |
| LV3 | PARTY MODE (`haste`: minis twice as fast, confetti) | CROSS HARP: two trains on crossing tracks |
| Below 25 HP | KAZOO ORCHESTRA: 12 mini-kazoos burst out at once and swarm the ring | RUNAWAY TRAIN: the track loops the whole ring and the train circles nonstop for 2 bars |

- **Likely shape (only if the honest fight gives it):** the harmonica's charges are telegraphed by its tracks, so the
  minis can swarm it from the side while it's locked on the rails; a train can plough through a whole swarm. KAZOO
  ORCHESTRA vs RUNAWAY TRAIN is a chaos finish either way.
- **Colours (in the table):** KAZOO red (255, 84, 72) (freed by the retired GUITAR; classic kazoo plastic), HARMONICA sky
  blue (128, 204, 255) (new; close-ish to the Piano's blue, which isn't in this fight).
- **Music, mood A (swamp blues stomp):** a dirty minor blues-rock groove: foot stomps, a heavy kick, a fuzz slide-guitar
  riff, a low organ drone; gritty, not happy. The **harmonica plays the bluesy lead** (bends, train chugs on its
  charges, the whistle on impact), the **kazoo the buzzing counter-line and squawks** (the minis buzz in a chord); the
  bed has no harmonica or kazoo of its own. `loop_bpm(122)` = 121.97 BPM (31 bars; bar = 1.968 s, FIGHT! at 3.94 s).
  Consistent loudness from frame 0 (full stomp on the title card; build with density). Leader fuller and louder, loser
  thinner and dirtier; the winner's solo over the stomp. (Not chosen: B dark bluegrass hoedown; C hyperpop chaos.)
- **VS title card:** KAZOO / VS / HARMONICA (no episode tag): a kazoo squawk, the thunder crack, a harmonica train
  whistle; panel patterns: kazoo body + membrane cap (red-lit) vs harmonica comb holes 1–10 (sky-lit).
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.94 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s KAZOO ORCHESTRA or RUNAWAY TRAIN · 44–52 s final exchange · K.O. ~49–53 s
  (slow-mo shatter into plastic shells / reed plates / comb) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s
  reassembly, doors shut, title slam = frame 0.
- **Flags:**
  - **Both instruments are new: build both models from reference photos found with Claude in Chrome** (as the user asked
    for the Viola), each a new file in `series/instrument-history/` in its own commit; voices: a buzzy hummed kazoo tone
    and a reedy harmonica with blow / draw and bends, own commits; bridge entries. Small instruments: scale them up
    inside the balls so they read (as the flute needed a pixel-art sprite).
  - The minis must read as part of the kazoo: smaller, red, linked to the main ball by a faint buzz line; the HUD and
    damage only follow the main ball.
  - New engine kinds additive, own commits; all earlier episodes replay identically. Re-tune for 40–60 % wins; honest
    seed search with `t_fight` = 2 bars and the usual hard windows. Effects from `reelkit.fx`; no full-screen flashes;
    safe-zone, flash, loop and levels checks must pass.
- **Effort:** ~75–90 min (two new models + voices, four new mechanics). Work in `projects/ballfight_kazoo_vs_harmonica/`.

## Board history (moved here from the status board, Oct 27)

- Status: idea (approved, mood A) · Session: — · Output: —
- Production cancelled by the user, Oct 27 (session_01CzTfqcRKd19SvcVTdGzjbd); the two models it had built were reverted at the user's request, so nothing from that session remains. A new session starts from scratch.
