# 36. BALLFIGHT #10: Trombone vs Tuba

Board row: `docs/IDEAS.md` (#36). Series rules: `docs/ballfight/RULES.md`.

## 36. BALLFIGHT #10: Trombone vs Tuba — APPROVED
**Direction (user, Oct 25):** "Draft a Trombone vs Tuba fight"; approved with mood C (brass house). The first crossover:
two fighters already built in their own episodes (#34 TROMBONE, #32 TUBA v2). All BALLFIGHT series rules apply: VS
title card on bar 0, 3·2·1 on bar 1, FIGHT! on bar 2; ability names as impact captions (first use, 4 s rule);
"COMMENT THE / NEXT MATCH-UP"; colours from the table; honest fight; 61.0 s; loop point = file boundary.
**Also the first episode with the HUD ability list** (series rule of Oct 25): this session builds the optional
`abilities` argument on `reelkit.ui.fight_hud` (own small commit; without it the HUD draws exactly as before).
**Rivalry:** who owns the low end? The two backbones of every brass band, each sure the other is "just the bass
section". The episode shows what the roster is for: both fight **exactly as they did in their own episodes.**
- **Post caption:** "Comment the next match-up 👇🎺📯"

| | TROMBONE (mint, left), from #34 | TUBA (indigo, right), from #32 v2 |
|---|---|---|
| Signature | Slide lance: aims and shoots the slide out on every beat; parries | OOM-PAH SWALLOW: "oom" inhales and swallows, "pah" spits the enemy into the ropes |
| Grows per landed hit | Lance reach (0.95 -> 2.5) | Suction reach (1.8 -> 3.0) |
| LV2 | SAD TROMBONE (shockwave + 0.6x slow for 1 s) | OOMPAH (swallows twice as often) |
| LV3 | POWER SLIDE (thrusts on 8ths) | BRASS WALL (brass dome shield) |
| Below 25 HP | PEDAL TONE (a giant bell drops and stomps) | SUBSONIC (the floor ripples like water) |

- **Likely shape (only if the honest fight gives it):** a reach battle. The lance must poke from outside the suction
  range, but every landed swallow stretches that range; SAD TROMBONE's slow is the trombone's escape, BRASS WALL blocks
  the lance completely.
- **VS title card:** the shared `projects/ballfight/title.py` with the existing TROMBONE and TUBA panel patterns (reuse
  #32 / #34's if present): TROMBONE slams in with a slide glissando blast, VS with the thunder crack, TUBA with a tuba
  blast, over unison house stabs.
- **Music, mood C (brass house):** a dark minor house beat (four-on-the-floor kick, offbeat hats, claps, a filtered
  minor pad and chord stabs), modern and TikTok-y; the fighters' hits are the brass stabs. `loop_bpm(126)` = 125.90 BPM
  (32 bars; bar = 1.906 s, FIGHT! at 3.81 s). Both fighters are low brass, so split the parts: **the tuba fighter is
  the bass line** (oom = root, pah = the fifth, as in #32) and **the trombone plays the tenor lead** above it (slides
  and smears, register kept up around 52–70); the bed has no bass synth and no brass of its own, so both sit clearly on
  top (band-level check). The leader's part fuller and louder, the loser's thinner and dirtier, as usual; the winner's
  solo over the house groove. Original and synthesised. (Not chosen: A dark New Orleans second line; B low-brass mambo.)
- **Crossover rules (new; for this and every later crossover):** both fighters come straight from `fighters.py` as last
  built (weapon, numbers, abilities, look, Pixel Pitch voice and idiom; the rebuilt trombone model from #34 v4). **The
  only per-episode lever is a damage handicap per fighter**, tuned to 40–60 % wins (add a per-fighter damage scale to
  the engine if it only has the shared one: additive, own commit). Mechanics, reach, timing and abilities never change
  per episode, so "how they fight" stays true everywhere. If damage alone can't balance a pair, flag it to the user
  instead of changing a fighter. All earlier episodes must still replay identically (`tools/ballfight_replay_hash.py`).
- **Beat sheet (61 s; the real fight decides after the count-in):** bar 0 VS title card · bar 1 count-in · FIGHT! at
  3.81 s · first clashes · 8–30 s the four unlocks (each named once; every use lights in the HUD list) · 32–44 s one
  below 25 HP fires PEDAL TONE or SUBSONIC · 44–52 s final exchange · K.O. ~49–53 s (hit-stop, slow-mo shatter into
  slide / tubing / bell parts) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors shut, title
  slam = frame 0.
- **Flags:** honest fight, seed search with `t_fight` = 2 bars and #27's hard windows (K.O. 49–53 s, >= 2 lead changes,
  winner HP <= 30, >= 5 ability captions >= 4 s apart, no 4 s without a hit). Two giant-bell moments exist (PEDAL TONE's
  dropped bell; the tuba's own bell): keep them visually distinct (PEDAL TONE's bell is mint-lit, the tuba's indigo).
  Effects from `reelkit.fx`; no full-screen flashes; safe-zone (taller HUD), flash and loop checks must pass.
- **Effort:** the lowest of any episode so far (~40–50 min): no new attacks and existing title patterns; the work is
  the handicap option (if needed), the HUD ability list, tuning + seed search, and the new music. Work in
  `projects/ballfight_trombone_vs_tuba/` importing the engine.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01FABTpA (2026-10-25) · Output: `ballfight_trombone_vs_tuba.mp4`
- v1 delivered (early sheet sent first). First crossover: both fighters straight from fighters.py; the only lever is the new opt-in engine handicap `Rules(dmg_side)` (own commit; all 9 finished episodes replay identically): TROMBONE x0.35 / TUBA x1.55 damage (the lance lands ~70 hits a fight, the swallow ~12), 300 seeds 49 % / 51 %, median K.O. 50.8 s. First HUD ability list: `reelkit.ui.fight_hud(abilities=...)` (own commit; without it 353 HUD input combinations draw pixel-identically): LV2 / LV3 / LOW rows under each bar, grey -> fighter colour with a sweep on unlock -> white-hot pill on every use. Honest search at 2 bars: 6 fits in 416, balance 48 / 52; seed **358** (106 ranked first but its 52.94 s K.O. left ~1 s for COMMENT THE NEXT MATCH-UP): the lance leads early, the swallows drag it back (lead changes 19.1 / 26.7 / 45.1 s), SUBSONIC 40.0, PEDAL TONE 44.3, the tuba gulps the trombone at 48.6 s and spits it into the ropes: TUBA wins at 49.10 s with 5.9 HP. Captions SAD TROMBONE 11.4, BRASS WALL 19.5, POWER SLIDE 24.3, SUBSONIC 40.0, PEDAL TONE 44.3. Music mood C (brass house, F minor, 125.90 BPM): no bass synth / brass in the bed; tuba = the bass line (oom root / pah fifth), trombone = the tenor hook; band check tuba +6.9..+8.6 dB, trombone +7.9..+11.8 dB over the bed. Checks: safe zone OK; CoC 0 on all 1830 frames; loopcheck PASS render 0.5x / mp4 1.2x (audio 1.2 dB, no click); flashcheck PASS (1 = the doors opening); opening -16.4 dBFS, build +3.2 dB. Waiting for feedback
