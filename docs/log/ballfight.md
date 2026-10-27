# Log: `projects/ballfight`

Append new entries at the end (newest last).

## BALLFIGHT #1: Guitar vs Piano (`projects/ballfight`)
- New series (#27), inspired by math.vsmath ability-ball fights: two glass balls with Pixel Pitch instruments inside
  fight in a boxing ring (turned 45 deg so the red corner is screen-left and the blue corner screen-right, matching the
  HUD) in a dark arena: hard spot overhead (can at the top of frame + a soft beam), red / blue corner-post lights, an
  LED barrier chasing on the beat, ten tiers of crowd silhouettes with rim light, phones and flashbulbs on big hits.
  GUITAR swings its neck (5 -> 24 frets, +1 per landed hit; parries keys), PIANO fires keys on every second beat
  (volley grows half a key per landed volley). Abilities: POWER CHORD (shockwave) / SHRED (spin x2), SUSTAIN (shield,
  a blocked neck bounces and reverses) / GLISSANDO (15-key arc), desperation FEEDBACK (wave) / GRAND SLAM (the lid
  slams on the predicted spot after 2 beats).
- Honest run, seed 88 (8 fits in 336 runs; balance GUITAR 44 % / PIANO 56 % over 313 decided): Guitar leads from 5 s,
  the maxed piano volleys catch up, lead changes at 31.6 / 36.0 / 44.4 / 48.0 s, GRAND SLAM lands at 44.4 and FEEDBACK
  fires 0.9 s later, PIANO wins at 51.3 s with 2.6 HP. Captions (first uses, >= 4 s apart): POWER CHORD 10.2, SUSTAIN
  15.3, SHRED 20.3, GLISSANDO 27.3, GRAND SLAM 43.4 (FEEDBACK named in the HUD status line).
- Reusable: `fight.py` (engine) + `fighters.py` (one table per fighter: weapon kind, scaling, 3 abilities, look, voice)
  + `reelkit.ui.fight_hud` (two fighting-game HP bars draining from the middle, damage trail, LV pips, status line).
- Rules lessons: a volley growing one key per landed volley maxed out by ~25 s and won every run by ~14 s; half a key
  per landed volley + one volley every 2 beats balanced it. Balance is touchy on blade damage (5 -> 7 swung guitar wins
  17 % -> 67 %): tune on a small grid (`tune.py`, 32-40 seeds per cell, ~1 min). Levels from damage dealt at
  14 / 36 put the unlocks near the beat sheet's 10-30 s targets. Fit rate ~2-3 % with the hard windows, so an
  early-stopping search finds 5+ fits in about a minute.
- Loop lessons: beat pulses (LED boards, corner glows, crowd hops) snap on at frame 0 because frame 0 is a downbeat;
  `timeline.beat_pulse` ramps each pulse up over the last 70 ms before the beat (seam 5.9x -> 2.2x together with an
  ease-out fly-in for the "3"). Audio: a riser + roll that build through the seam (12 ms past it, no fade) and the
  frame-0 count hit 4 ms in -> 4.2 dB step, no click (was 7.8 dB + click).
- Sound: section automation after the compressor (build +3.4 dB); every hit snapped to the nearest 16th in the
  attacker's own voice; a fighter's part high-passes and saturates as its HP drops, the leader gets an octave double.

## The episode kit (Oct 27): one config per episode (`projects/ballfight/kit/`)
- Task (user): turn BALLFIGHT into a reusable kit so new episodes need far less code. Read #41 (crossover) and #37 (new
  fighters) only. Built `projects/ballfight/kit/` + `python -m ballfight <step> <episode>` (new, preview, tune, search,
  sim, sheet, audio, check, render, verify, post, hash). An episode is one `episode.py` (fighters, sides, SEED, handicap,
  rules, BPM, mood, key, chords, lines, solos, title hits, caption); `kit/episode.py` derives the timeline from it.
- Each fighter's look + sound moved to `kit/fighters/<name>.py` (Look / Sound hooks at fixed points of #41's pipeline,
  listed in `kit/fighters/base.py`): FLUTE, TROMBONE (from #32 / #34 / #41), CLARINET, OBOE (#37). Moods: `big_top`
  (#41), `baroque_storm` (#37), ported as they were. The finished episode folders are untouched.
- Proof, #41 as `examples/flute_vs_trombone/episode.py` (61 lines vs 2,799 in #41's folder): seeds 392/1/2/3/500 hash
  the same as #41's sim; 33 frames pixel-identical to #41's render.py; audio.wav bit-identical (5,857,280 samples); every
  check passes with #41's numbers (band trombone +6.5..+7.8 dB, flute +12..+16; intro -1.5 LU; seam 0.4x). #37 as
  `examples/clarinet_vs_oboe/episode.py` (39 lines): same hashes, bit-identical audio, 22/25 frames identical (a few px
  where the zip and the beam overlap; the glide home bows 0.1-0.2 units: #41's fix, now in every episode).
- Lesson: `reelkit.ui`'s caption puff seeds from Python's salted `hash(str)`, so the same frame differed between runs
  (100-185 px under captions, #41's own renderer too) and `--frames` patches could not match a full render. The kit
  pins PYTHONHASHSEED=0 (a one-line `zlib.crc32` fix in reelkit.ui would remove it for every project: needs the
  user's OK, reelkit is shared).
- Timings on 2 cores (#41): tune 300 seeds 52 s, search 1..6000 --stop 8 165 s (the same 8 fits by seed 968, 392 first
  at 7.78), audio 9 s, render 431 s + one loop-clean encode 227 s (the kit skips reelkit.video's own encode), check
  4.5 min, verify 46 s; tune + search + render ~15 min. A fighter preview (7 s, 360x640, with sound) takes ~75 s.
- Replay guard: all 95 lines unchanged; `tools/ballfight_replay_hash.py` now also runs the kit configs (KIT_EPISODES),
  which print the same lines as #41 and #37.
