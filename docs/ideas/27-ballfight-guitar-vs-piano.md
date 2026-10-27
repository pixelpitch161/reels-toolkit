# 27. Instrument BallFight: Guitar vs Piano

Board row: `docs/IDEAS.md` (#27). Series rules: `docs/ballfight/RULES.md`.

## 27. Instrument BallFight: Guitar vs Piano — APPROVED
**Direction (user, Oct 23):** new format inspired by the math.vsmath ability-ball fights the user shared
(https://www.instagram.com/reels/Ded-QzGznYV/). Mood A (drift phonk). Habits to borrow (user approved all): a 3-2-1
FIGHT countdown at frame 0; ability names as impact captions; status in small text inside the safe zone; "Comment the
next match-up" in every post caption, then make the most-liked request; famous rivalries over obscure ones.
**Keep mid-fight captions to what's necessary** (user): ability names only. No facts, no commentary like #23's
"DEAD END / SO CLOSE", no "who's winning". Standing notes: 61.0 s; instruments on screen; impact captions; loop point
= file boundary; new safe zone (bottom HUD down at y 500 internal, left of the button column).
**Concept:** two glass balls, each with a low-poly instrument inside, fight in a boxing ring. Each fights with its
instrument and every hit plays it, so the fight is the duet. Every landed hit grows that fighter's weapon, so the
fight escalates by itself. Episode 1: the biggest rivalry in music, guitar vs piano.
- **Hook (frame 0):** a giant **3 · 2 · 1 · FIGHT!** in the ring, one count per beat. It is also the band's count-in:
  guitar chord + piano octave + 808 on each count, the drop lands on FIGHT (1 bar = 1.85 s). Top caption "GUITAR /
  VS PIANO" (6 + 8 chars). Both balls rev in their corners: loud from frame 0.
- **Post caption:** "Comment the next match-up 👇🎸🎹" on every episode; the next episode is the most-liked request.

| | GUITAR (red corner) | PIANO (blue corner) |
|---|---|---|
| Weapon | Neck spins around the ball like a sword; +1 fret length per landed hit | Fires keys as projectiles on the beat; +1 key per volley per landed hit |
| Hit sound | Power-chord stab | Chord stab; each key in a volley climbs the scale |
| Unlock 1 | POWER CHORD: shockwave that knocks the piano back | SUSTAIN: shimmering shield for 2 bars |
| Unlock 2 | SHRED: spin doubles for 4 s, rapid 16th notes | GLISSANDO: sweeping arc of keys across the ring |
| Below 25 HP | FEEDBACK: screeching wave | GRAND SLAM: the lid slams down on an area |

- **Captions (impact):** mid-fight only the ability names, in the fighter's colour, about 6 in the whole fight, never
  two within 4 s. Besides those: the title, the countdown, "K.O." then "(WINNER) WINS", and the end card "COMMENT THE /
  NEXT MATCH-UP".
- **Status (small text, inside the safe zone):** HP as a small number inside each ball; two fighting-game HP bars at
  the bottom of the safe zone ("GUITAR 64", "PIANO 71") with level pips LV1–3, left of the button column. Build it as
  a reusable `reelkit.ui` fight HUD piece. The names on the bars say which instrument is which.
- **Music, mood A (drift phonk):** cowbell riff, sliding 808 bass, menacing, minor key; the sound of TikTok fight
  edits. `loop_bpm(130)` = 129.84 (33 bars; 1 bar = the countdown). The two instruments are the leads: every hit plays
  the attacker's instrument (pitch in key, timing snapped to the nearest 16th in the audio, ≤60 ms). You can hear who
  is winning: the leader's part gets fuller and louder, the loser's thins and distorts as its HP drops. The winner's
  victory solo is an original motif in its own idiom. Original and synthesised. (Not chosen: B retro arcade boss
  theme; C taiko war drums + low strings.)
- **Scene:** a low-poly boxing ring floating in a dark arena, seen from a 3/4 top-down camera; one hard spotlight
  overhead (key light) plus red and blue corner lights (local lights); crowd silhouettes with phone lights (small
  life); flashbulbs on big hits. The ropes are the walls: they wobble on bounces and thud on the bass. Time-saving
  fallback: `reelkit.backdrops.VibrantVoid`.

**Beat sheet (61 s; events after 2 s are targets: the real fight decides, so choose a seed whose run fits):**
| Time | Visual | Caption (impact) | Sound |
|---|---|---|---|
| 0–1.85 | Balls rev in corners; giant 3·2·1 in the ring | GUITAR / VS PIANO, then 3·2·1·FIGHT! | Count-in: chord + octave + 808 per count; drop on FIGHT |
| 1.85–8 | Launch, first clashes, HP ticking | — | Full groove; every hit plays its instrument |
| 8–20 | Neck lengthens, volleys grow, first unlocks | POWER CHORD (~10), SUSTAIN (~15) | Leader's part swells |
| 20–32 | Lead swings back and forth; ring crowded with keys | SHRED (~24), GLISSANDO (~30) | Shred 16ths vs climbing key runs |
| 32–44 | Both weapons huge, faster, flashbulbs; one dips below 25 HP | FEEDBACK or GRAND SLAM (~40) | Loser's part thins and distorts |
| 44–52 | Final exchange, HP in single digits | — | Riser, call and response |
| ~52 | KO: brief slow-mo; the loser shatters into keys / strings | K.O. → (WINNER) WINS | Big hit |
| 52–58 | Winner's victory solo; crowd lights up | COMMENT THE / NEXT MATCH-UP | Winner's solo over the groove |
| 58–61 | Loser reassembles; both glide back to corners; HP refills | — | Pickup; "3" lands at 61.0 = frame 0 |

- **Flags:** honest fight, simulated and never hand-edited. Balance the two so each side wins ~40–60% across seeds;
  pick a seed with ≥2 lead changes, a close finish and a KO ~49–53 s; early-stopping search (`--stop 5`). Loop point =
  file boundary (the reassembly is the run-up; "3" is frame 0). Safe-zone, flash (flashbulbs, feedback wave) and loop
  checks must pass. Generic instruments only (models + voices from Pixel Pitch `series/instrument-history/`: import or
  copy, don't edit the series files); no brands.
- **Series:** "BALLFIGHT #1". Define each instrument's weapon, scaling and abilities in one table / config so later
  episodes are mostly a new ability pair on the same engine. Next match-ups if the comments are quiet: Violin vs
  Drums, Trumpet vs Sax, Bagpipes vs Accordion ("most hated"), Electric vs Acoustic.
- **Effort:** moderate for episode 1: a small 2D fight engine (top-down, constant-speed balls, rope walls, weapon
  hitboxes, projectiles), ability effects, the ring, the fight HUD. Instrument models and voices reused. Later
  episodes ~30–40 min. Work in `projects/ballfight/` with the engine in its own module so episodes import it.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_016aCf6G (2026-10-23) · Output: `ballfight_guitar_vs_piano.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest seed 88 (8 fits in 336 runs; balance GUITAR 44 % / PIANO 56 %): Guitar leads from 5 s, lead changes 31.6 / 36.0 / 44.4 / 48.0 s, GRAND SLAM 44.4 then FEEDBACK 0.9 s later, PIANO wins at 51.3 s with 2.6 HP. Captions = ability names only (POWER CHORD 10.2, SUSTAIN 15.3, SHRED 20.3, GLISSANDO 27.3, GRAND SLAM 43.4; FEEDBACK named in the HUD status line, 4 s rule). Changes vs backlog: the piano's volley grows half a key per landed volley and fires every 2 beats (one key per landed volley won every run by ~14 s); levels from damage dealt (14 / 36); K.O. = 0.35 s hit-stop + slow-mo shatter; solo bars 29-31. Reusable: fight.py + fighters.py (projects/ballfight) + new reelkit.ui.fight_hud. Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS on render + mp4 (seam 2.4x, 4.1 dB, no click); flash 0 general; opening -19.7 dBFS, build +3.4 dB. Waiting for feedback
