# Log: `projects/ballfight_viola_vs_violin`

Append new entries at the end (newest last).

## #42 BALLFIGHT #16: Viola vs Violin (session_01CzTfqcW39D, Oct 27)
- New fighter from photos in the user's Chrome (Claude in Chrome): Commons' API from a Commons page (same origin) for
  the file lists, candidates drawn into a canvas (`crossOrigin`) and measured per pixel row. "Bratsche.jpg" (public
  domain) has a 16.5" viola's front AND side view on white: outline, corners, f-holes, bridge, tailpiece, fingerboard
  from the front; rib depth + arching from the side. "Violin-Viola.jpg" gave the size ratio in one photo (1.09-1.18x).
- Size has to be designed into the sprite, not just the model: `model_faces` normalises every model to a unit sphere,
  and the #28 violin's model includes its bow (frog and tip boxes far out), which shrinks the violin inside its ball.
  The viola model has no bow, the bridge sizes it x1.12, and its view is three-quarter on (yaw 34) so the deep ribs
  show. Above r ~ 25 px the x1.12 reaches past `sprites.render_sprite`'s 3 px crop margin: the episode renders the viola
  with a wider crop (`render.render_padded`, also the title card's close-up).
- Engine (4 small opt-in commits + the roster entry; guard identical on all 80 runs each time): `rests` (a `guard` factor
  in `damage()` while resting, records REST / NOTES), `double_stop`, `melody`, `joke` (#37's stun state applied directly,
  plus a note dropped from above). Notes land ~84 %: a slow wheel as big as the ball rolling a whole bar is hard to dodge
  for constant-speed balls. A "topple at the ropes" option was tried: most crushes happen on the first pass, so it barely
  changed that and was dropped (not committed).
- Tuning: the rests halve the bow most of the time, so the violin's handicap is big (x2.1 vs the viola's x0.75). Flat
  note damage (3.0, +0.04 a crush) lets the viola lead early and the violin's spin growth + TREMOLO claw back mid-fight.
  Fits ~0.3 %: two ranges (5,929 runs) for 18 fits; seed 8219 was the top score of both and the only top one where
  VIOLA JOKE wasn't eaten by the 4 s caption rule.
- Picture: an upright rolling wheel is a sliver from the 58-degree camera whenever it rolls toward / away from it, so
  the whole note faces the viewer (the engraved notehead's oval and tilted hole), still rolling: turning the oval bobs it
  and spins the hole, signed by its screen direction. Drawn 1.3x its hit radius (contact at r + 0.9 R), so the crush
  lands as the glass touches it. The counted "1 2 3" moves beside the rest glyph when a caption is up (safezone overlap).
- Music: the viola was only +1.6 dB over the cellos in its band before it led; +2 dB on its notes / plucks and lighter
  cello stabs gave +3.7..+10. Levels passed first time with no automation (spread 2.0-2.4 dB). The crowd laugh is
  synthesised: 34 voices of pulse-train "ha"s through /a/ formants, random onsets / pitches / pans.
- Checks: levels PASS (wav + mp4, spread 2.4 dB); band viola +3.7..+10, violin +10..+17 dB; no silent stem windows; safe
  zone OK; CoC 0; flash test 0 on 39.5-51.5 s, flashcheck PASS (the doors); loopcheck PASS render 0.4x / mp4 1.0x. Render
  9.3 min + mux. Guard: 90 runs + #42 (95).
- Feedback: (pending)
