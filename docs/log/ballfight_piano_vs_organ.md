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
