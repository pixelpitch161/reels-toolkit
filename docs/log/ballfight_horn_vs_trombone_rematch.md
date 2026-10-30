# Log: #54 BALLFIGHT: Horn vs Trombone, THE REMATCH (`projects/ballfight_horn_vs_trombone_rematch/`)

## Oct 30: made (session_01CzTfqd)
- Episode-kit crossover, a viewer request on #34's reel ("RIGGEDD!! RUN IT BACK!!"): HORN v3 (left, copper) vs TROMBONE
  (right, mint), both exactly as in `fighters.py`; sides set in `episode.py` only (the reverse of #34). No episode number.
- Title card: new opt-in `MIDDLE = (word, angle)` on the shared `title.py` (own commit): REMATCH in gold along the
  diagonal crack ('divider' = 39.8 deg), turned with nearest-neighbour rotation before the outline (crisp pixels), at the
  biggest scale (7) that fits the safe zone out of the button column and clear of both names; streaks and dust run along
  it, its safe-zone notes are narrow strips. Without MIDDLE every card is unchanged: 13 title frames hashed identical
  for the #41, #38, #53 and #49 configs before / after.
- HORN wasn't in the kit: ported from #34 v5 into `kit/fighters/horn.py` (#34's styles copied unchanged, its render
  layers as Look hooks, its audio part as Sound). Check config `examples/trombone_vs_horn`: replay hash identical to
  #34's own sim (140, 1; added to the replay guard); 12 fight frames pixel-identical to #34's renderer with the UI off
  (ghosts, rings, STOPPED HORN, HORN RIP, the glove's wind-up and flick, afterimages); shatter frames differ only by a
  few levels from the kit trombone's own #41 additions (the hovering bell's light, the stomp's shake); the horn bus
  bit-identical to #34's up to the reassembly (the kit's tail is half a bar earlier). Kit additions (additive): ECHO_G /
  ECHO_N / FLICK in REC_KEYS, the flick's HUD hold, `Look.canvas_overlay` (the copper thread). First frame diff found
  the raised blast rings missing: drawn as the first painter item (depth +inf = after the back ropes, as #34).
- Mood `kit/moods/brass_trailer.py`: #34's bed ported (strings / braam / tremolo buses bit-identical, taiko to 2e-16
  vs #34's audio.py); opt-in `TRAILER = dict(sequel=True)`: contrabass octave, cello pad, braams + subs every second
  bar, the taiko ensemble from the start, war-drum fills, a braam + sub under REMATCH, the full trailer under the solo
  (and bridging stinger -> solo), big taiko + double braam swell into the seam. D minor with a Neapolitan Eb in the
  8-bar loop, loop_bpm(122) = 121.97, 31 bars, FIGHT! at 3.94 s. Horn = the noble call (LINES, one note a beat; the
  echoes are its delay), trombone = the riff (role 'riff', 8ths), trombone bus 2.3 (MIX) to clear the darker strings.
- Tuning (the only lever, frozen): DMG_SIDE = (1.04, 1.08) at #34's DMG_SCALE 0.72: 300 seeds, HORN 53 % / TROMBONE 47
  % of 266 decided, median K.O. 49.5 s (unhandicapped: 53 / 47 but median 52.3 s).
- Seed: search 1..6000 --stop 8: 8 fits in 729 runs (51 / 49 decided). Picked 630, ranked first and a different story
  from #34 v5 (there: FINAL FLICK, PEDAL TONE, the trombone wins at 49.2 s): FINAL FLICK lands at 45.8 s and flings it
  into the ropes, PEDAL TONE's bell crushes the horn for 18.7 at 47.2 s to retake the lead, the horn's echoes finish it at
  50.28 s with 2.8 HP. Runner-up 345 replayed #34's timeline almost beat for beat, so story ruled it out (not the winner).
- Levels lesson: #34's light victory bed relied on its section automation; under the kit's one master the end sat ~6 dB
  under the card. The sequel keeps the trailer full after the K.O.; levels PASS (spread 4.4 dB).
- Checks: safe zone, focus (CoC 0), levels, band (horn +8.5..+10.6 dB, trombone +3..+5 dB), flash (max 14.5 %), loop
  (seam 0.4x, audio 2.9 dB) all PASS before the render.
