# Log: `projects/ballfight_trombone_vs_tuba`

Append new entries at the end (newest last).

## BALLFIGHT #10 (#36): Trombone vs Tuba, the first crossover + the HUD ability list
- Crossover rule in practice: the fighters come straight from `fighters.py` and the renderer imports both episodes'
  draw functions (`styles.py` loads #34's and #32's styles modules by path) instead of copying them. The engine only had
  the shared `dmg_scale`, so `Rules(dmg_side=(left, right))` went in as its own opt-in commit (replay guard identical
  for all 9 finished episodes). The pair is very lopsided in hit count (lance ~70 hits, swallow ~12), so the handicap is
  big: TROMBONE x0.35, TUBA x1.55 (49 % / 51 % over 261 decided runs, median K.O. 50.8 s). Damage alone balanced it.
- HUD ability list: `reelkit.ui.fight_hud(..., abilities=(left_rows, right_rows))`, rows = dict(tag, name, state, glow,
  sweep), in its own commit. Checked "draws exactly as before" by hashing 353 HUD input combinations (image + the
  safe-zone notes) before and after. Two look fixes from the first test: the tag needs a real space ("LV2SAD" read as one
  word), and the dark text outline filled the 1 px gaps between letters, chopping the pill into pieces: while lit, the
  outline shifts to a deep shade of the fighter's colour and all pills draw before any text. The episode builds the
  rows from the run's `level` / `ability` events (`render.ability_rows`): held lit while a shield / haste lasts, a slam
  until it lands; frozen through the hit-stop; back to grey on the glide home. The 18 px taller HUD stayed clear of the
  action (the balls never got below internal y 417), so no ring nudge.
- Seed pick: the top-scoring fit (106) had its K.O. at 52.94 s, which leaves ~1 s for COMMENT THE NEXT MATCH-UP and a
  5-beat solo; took the second (358, K.O. 49.10 s). Worth adding "K.O. + 3.65 s caption gets >= 3 s" to the score.
- Merging two renderers: the slam kind is shared (PEDAL TONE and SUBSONIC are both `slam`), so every slam consumer
  (the giant bell, its target decal, the "under the bell" HP hide, shimmers, fx, the focus audit) filters by owner.
  #32's TUBA title panel was centred on the left ball; it's recentred on this episode's right-hand ball. PEDAL TONE's
  bell is lit mint (a flare during the hover + a halo under the lip) so it reads apart from the tuba's own bell.
- Focus audit: the hovering PEDAL TONE bell (up to 8 units above the canvas) is 4.8 px soft, as it was in #34; the audit
  now checks its rim once it stomps (0 px), and the subject stays 0 px on every frame.
- Music (mood C, brass house): with no bass synth the kick had to make room for the tuba: a 0.16 s body high-passed at
  48 Hz and a lower send lifted the tuba from +5 to +8 dB over the bed in 40-260 Hz.
- Checks: safe zone OK; CoC 0; loopcheck PASS render 0.5x / mp4 1.2x (audio 1.2 dB, no click); flashcheck PASS (1, the
  doors); opening -16.4 dBFS, build +3.2 dB. Render 9 min + 3 min mux.
- Feedback: (pending)
