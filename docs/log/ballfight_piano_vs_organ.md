# Log: `projects/ballfight_piano_vs_organ`

Append new entries at the end (newest last).

## #45 BALLFIGHT: Piano vs Organ (`projects/ballfight_piano_vs_organ`, session_01CzTfqdN2i3, Oct 27)
- Episode-kit episode (one config, `episode.py`), the debut of PIANO v2 (PIANO ROLL) against the ORGAN as built in #35
  (ported into the kit for it). Mood A, prog-rock keyboard duel (`kit/moods/prog_duel.py`), A minor, 133.77 BPM.
- PIANO v2 in the engine (own commits, replay guard identical before / after each): weapon kind `roll` (a player-piano
  roll along the side through the fighter's own corner and the back corner; on the 8th grid every row's holes fire
  note-shots from their pitch lanes, straight across blended toward the enemy by `aim`; +holes per landed row), ability
  kinds `sustain` (the next rows' shots fly a little, hang like mines, staggered by hole index, then re-aim) and
  `runoff` (88 KEYS: every lane at once straight across, then the roll is re-threaded). The rows come from the config's
  line: `kit/story.roll_rows` (melody, its octave below, the chord's tones under it) -> `Rules(roll_rows=, roll_start8=)`,
  so the holes ARE the piano's notes in the music. Roster: PIANO = v2; #27's v1 is frozen in `projects/ballfight/sim.py`
  (patched in at import; #27's render / audio / tune import sim first), like #39's BARI SAX v1.
- Look (`kit/fighters/piano.py`): the roll is a canvas decal on the apron (cream paper as a light multiplier over the
  apron's albedo), so the ropes, shadows and attack lights sit on it; upcoming rows are visible before they fire (the
  melody coming), holes flash blue at the brass read bar; key-shaped shots (sharps black) with trails; mines pulse over
  blue rings; 88 KEYS fires a wall of keys and the paper runs off in shreds, the new roll slides in from the spool.
- ORGAN port (`kit/fighters/organ.py`, done by a helper agent in parallel): #35's draw code copied unchanged, layers and
  sound moved into Look / Sound, side-agnostic (the organ is on the right here), role 'bass' = a pedal note under every
  pipe row and a pedal climb in the last bar (any other role = #35's part, bit-identical). New optional Look hooks
  `light()` (the CATHEDRAL dome lights the scene), `fx_end()`, `fringe_size()`, `block_burst` / `block_spark` (defaults =
  the old constants; kit example sheets byte-identical). Check against #35 (scratch kit config of #35, seed 237): replay
  hash identical; with a matched harness (the harp's drawn layers off in #35, a stub harp carrying only its effects in
  the kit, #35's SY 280 / beat-pulse ramp / reassembly time) 19 frames pixel-identical (rows, landed rows, CATHEDRAL,
  PULL ALL STOPS, TOCCATA, the K.O., the solo pipes, seed 2's organ shatter, the rev / seam); the organ + solo buses
  bit-identical. The plain kit render differs from #35 only through kit series rules (camera SY 288, 0.12 s beat ramp,
  the reassembly bar), which also drops #35's last solo note.
- Preview clip sent before tuning (65 s to make on 2 cores).
- Tuning (300 seeds each cell; the PIANO's own numbers are new, so they are frozen in the roster; the ORGAN's only lever
  is its handicap): at full damage both sides finished in ~29 s; the piano lands ~190 hits a fight to the organ's ~40,
  so the piano is steady chip damage and the organ bursts. Growth rate barely moves lead changes (~0.6 / run at every
  cell); frozen PIANO dmg 0.5 (GLISSANDO 0.25, 88 KEYS 0.45), grow 0.25, ORGAN x0.55: 50 % / 50 %, median K.O. 51.0 s,
  fit rate 1.0 % (the >= 2 lead changes and >= 5 captions windows do the filtering). Search 1..8000 --stop 8: 8 fits in
  1,097 runs (2 min), all ORGAN wins; seed 638 ranked first.
- Checks (`python -m ballfight check`, 191 s): safe zone PASS, focus 0 px (read bar + spool are focus points), levels
  PASS (opening +0.2 dB vs the ending, spread 2.4 dB), band PIANO +9.4..+13.3 dB / ORGAN +3.7..+7.5 dB over the bed,
  intro -1.1 LU vs the fight, flash 0 general (max 14.0 % at the 88 KEYS / TOCCATA beat), loop PASS (seam 0.4x).
- Note: `python -m ballfight render` in a background shell started from another folder failed with "No module named
  ballfight"; `python /path/to/projects/ballfight render <episode>` works from anywhere.
