# 28. BALLFIGHT #2: Drums vs Violin

Board row: `docs/IDEAS.md` (#28). Series rules: `docs/ballfight/RULES.md`.

## 28. BALLFIGHT #2: Drums vs Violin — APPROVED
**Direction (user, Oct 23):** #27 Guitar vs Piano "turned out amazing and I think will be a winner": make the next
episode, Drums vs Violin. Mood B (dark Balkan fire). Everything in #27's direction still holds: 3-2-1 FIGHT count-in
at frame 0, mid-fight captions = ability names only (first use, never two within 4 s, repeats in the HUD status line),
status in small text inside the safe zone, "Comment the next match-up" in the post caption, honest fight, 61.0 s, loop
point = file boundary.
**Concept:** brute force vs finesse on #27's engine, ring, HUD and countdown (`projects/ballfight`: `fight.py`,
`fighters.py`, `reelkit.ui.fight_hud`). The two fight in completely different ways, so it isn't a reskin. **The
drums keep the beat:** the drum ball attacks on every beat, so its attacks are the groove, and the violin plays the
melody over it. Whoever is losing, you hear their half of the band fall apart.
- **Hook (frame 0):** the same 3 · 2 · 1 · FIGHT! count-in (one bar = 1.69 s at this tempo). Top caption "DRUMS / VS
  VIOLIN" (5 + 9 chars), with the "BALLFIGHT #2" tag.
- **Post caption:** "Comment the next match-up 👇🥁🎻"

| | DRUMS (orange, left corner) | VIOLIN (violet, right corner) |
|---|---|---|
| Style | Close-range area control, heavy hits | Long reach, fast, precise |
| Weapon | Beat pulse: a drumhead ring blasts out from the ball on every beat; reach grows per landed pulse (new weapon kind `pulse`) | The bow, spun around the ball like a rapier: long, thin, light damage; spins faster per landed hit, slow strokes -> frantic spiccato (blade with a new spin-growth option, style `bow`) |
| Hit sound | Kick on 1 and 3, snare on 2 and 4; landed hits add tom fills | Short bowed notes climbing the scale |
| Unlock 1 | DRUM ROLL (`haste`): pulses on every 16th for a bar | PIZZICATO (`arc`): a fan of 5 plucked-string darts |
| Unlock 2 | CRASH CYMBAL (`shield`): spinning cymbal; the bow bounces off it | TREMOLO (`haste`): bow blurs at 2.5x for 3 s |
| Below 25 HP | DOUBLE KICK (`slam`): a giant kick pedal stomps the predicted spot | SCREECH (`wave`): the beginner-violin squeal as a ring-wide wave |

- **Natural story (only if the honest fight gives it):** drums bully early while the bow is slow; every landed bow hit
  speeds it up, so the violin can claw back mid-fight. SCREECH and DOUBLE KICK should get comments either way.
- **Captions (impact):** ability names only (as #27), plus the title, the countdown, "K.O." then "(WINNER) WINS", and
  "COMMENT THE / NEXT MATCH-UP".
- **Status:** `reelkit.ui.fight_hud` as in #27 (HP bars "DRUMS" / "VIOLIN", LV pips, status line), HP numbers in the balls.
- **Music, mood B (dark Balkan fire):** harmonic minor, upright-bass oom-pah, an accordion-ish pad, fiery, not happy:
  the violin's home turf. `loop_bpm(140)` = 141.64 BPM (36 bars in 61 s). The bed has **no drums of its own**: the
  drum fighter is the kit, the violin is the lead. As the drums lose HP the beat thins (fills gone, then the snare, then
  a lonely kick); as the violin loses HP its melody gets scratchy and sparse; the leader is fuller and louder. The
  winner's solo: a drum solo or a violin cadenza (original). Every hit snapped to the nearest 16th in the audio. (Not
  chosen: A drift phonk; C cinematic trailer.)
- **Scene:** #27's ring and arena reused (DRUMS left, VIOLIN right). **Colours (user, Oct 23): every instrument has
  its own ball colour, not red vs blue every time:** DRUMS orange (255, 146, 40), VIOLIN violet (176, 98, 255); the
  corner lights, LED barrier sides, HUD, captions and effects take those colours (see the series colour table below).

**Beat sheet (61 s; #27's proven shape; events after 2 s are targets, the real fight decides):**
| Time | Visual | Caption (impact) | Sound |
|---|---|---|---|
| 0–1.69 | Balls rev in corners; giant 3·2·1 | DRUMS / VS VIOLIN, then 3·2·1·FIGHT! | Count-in on the drum fighter's sticks + violin chop; drop on FIGHT |
| 1.69–8 | Launch; pulses vs bow, HP ticking | — | The fight plays the groove: pulses = kick / snare, bow hits = melody |
| 8–30 | Pulse reach grows, bow speeds up; unlocks | DRUM ROLL, PIZZICATO, CRASH CYMBAL, TREMOLO (first uses, >= 4 s apart) | Leader fuller |
| 32–44 | One dips below 25 HP | DOUBLE KICK or SCREECH | Loser's half thins |
| 44–52 | Final exchange, HP in single digits | — | Riser, call and response |
| ~49–53 | K.O.: hit-stop, slow-mo shatter (drumheads / strings) | K.O. → (WINNER) WINS | Big hit |
| then–58 | Winner's solo under the spot | COMMENT THE / NEXT MATCH-UP | Drum solo or violin cadenza over the bed |
| 58–61 | Loser reassembles, both glide to corners, HP refills | — | Pickup; "3" lands at 61.0 = frame 0 |

- **Flags:** honest fight, never hand-edited. Re-tune once (`tune.py`: 40–60 % wins, ~50 s median K.O.), freeze the
  rules, then `seed_search.py --stop 5` with #27's hard windows (K.O. 49–53 s, >= 2 lead changes, winner HP <= 30,
  >= 5 ability captions >= 4 s apart). Rules depend on the tempo (beat length, `t_fight` = 1.6945 s): pass them from
  this episode's timeline. New engine pieces (`pulse` weapon kind, blade spin growth, render styles `drumhead` and
  `bow`) go into the shared engine in small commits so later fighters can use them; don't break #27's run (seed 88
  must replay identically). If the full drum kit doesn't read at ball size, show a snare + crossed sticks. Flash
  (cymbal glints, pulse rings), safe-zone and loop checks must pass.
- **Effort:** ~45–60 min (two new weapon types); work in `projects/ballfight_drums_vs_violin/` importing the engine
  (see `projects/ballfight/README.md`, "Making the next episode").

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01KeEL7s (2026-10-23) · Output: `ballfight_drums_vs_violin.mp4`
- v1 delivered with caption + tags (early sheet sent first), re-rendered in the new series colours (DRUMS orange, VIOLIN violet) after the colour-table direction landed mid-production (red/blue cut kept in check/). Honest seed 568 (6 fits in 932; balance DRUMS 57 % / VIOLIN 43 % over 767 decided): Drums edge the opening, the speeding bow takes over at 18.2 s and builds a 62-26 lead, DOUBLE KICK 37.7 starts the comeback, SCREECH 43.6, DRUMS win on the last hit at 50.91 s with 2.8 HP. Captions = ability names only (DRUM ROLL 12.7, TREMOLO 22.0, CRASH CYMBAL 32.6, DOUBLE KICK 37.7, SCREECH 43.6; PIZZICATO 14.0 in the status line, 4 s rule). Engine (shared, additive): weapon kind `pulse`, blade `spin_grow`, `dur_beats`; #27 seed 88 replays identically. Music: drum-less Balkan bed (D harmonic minor, 141.64 BPM); pulses = the kit (thins with the drums' HP), bow quarter turns = the melody (1 -> 9 notes/s as the bow speeds up). Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS on render + mp4 (seam 1.8x, 0.1 dB, no click); flash 0 general; opening -17.8 dBFS, build +3.2 dB. Waiting for feedback
