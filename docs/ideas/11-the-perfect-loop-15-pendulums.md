# 11. The Perfect Loop (15 pendulums)

Board row: `docs/IDEAS.md` (#11). Series rules: none.

## 11. The Perfect Loop (15 pendulums) — APPROVED
- **Hook:** "15 PENDULUMS" / "1 PERFECT LOOP" (2 lines, in the first 2 s).
- **Science:** 15 uncoupled pendulums (no motor, magnets or links). The longest swings 51 times in 60 s;
  each shorter one swings once more, up to 65. They drift into travelling waves, standing waves and apparent
  random motion, split into two opposite halves at 30 s, and at exactly 60 s every pendulum has done a whole
  number of swings, so they're back in line. T = 2π√(L/g), so lengths run ~34 cm → ~21 cm (calc).
  Group milestones (calc): adjacent pendulums slip one full swing apart every 60 s, so at t = 60/n s they form
  n groups: 6 @10 s, 5 @12, 4 @15, 3 @20, 2 @30, then 3 @40, 4 @45, 5 @48, 6 @50, one line @60.
  Source: [Harvard Natural Sciences Lecture Demonstrations](https://sciencedemonstrations.fas.harvard.edu/presentations/pendulum-waves).
- **Music:** each pendulum plucks its own note every time it reaches the far side (once per full swing).
  Longest = lowest, across 3 octaves of a pentatonic scale so "chaos" stays consonant. The groups ARE the
  rhythm: 2 groups = eighths, 3 = triplets, 4 = sixteenths, 5 = quintuplets, 6 = sextuplets. First half: the
  subdivisions slow into the halftime call-and-answer at 30 s. Back half: they speed back up 3 → 4 → 5 → 6
  into one unison chord at 60 s. Original bass + pad change chord with each pattern. All synthesised.
- **Scene:** low-poly clockmaker's attic at sunset: round window (low sun = key light), oil lamps (warm local
  lights), ticking wall clocks, a cat and dust motes (small life). A wooden frame holds 15 glowing lantern-bobs
  in rainbow colours, one per note, each flashing as it plays. Floor seen from above, so the bobs' shadows draw
  the wave a second time. Escalation is the physics: line → snake → groups → chaos → halves → groups → line.
- **Payoff / ending:** the back-half acceleration lands on the 60 s realignment: all 15 hit together, which is
  frame 0, so it's a true seamless loop.

**Beat sheet (60 s; physics clock in real time, t = 0 = release):**
| Time | Pendulums (calc) | Caption | Sound |
|---|---|---|---|
| 0–4 | All 15 released in a line, one flash | 15 PENDULUMS / 1 PERFECT LOOP | Unison chord, then rippling arpeggio |
| 4–8 | They peel into a travelling snake | 51 → 65 SWINGS / PER MINUTE | Arpeggio ripples |
| 8–12 | 6 groups (10 s), then 5 (12 s); length labels | LONGER = SLOWER | Sextuplets → quintuplets |
| 12–16 | 4 groups (15 s) | NO MOTOR / NO MAGNETS | Sixteenths |
| 16–22 | 3 groups (20 s) | 3 GROUPS / = TRIPLETS | Triplets |
| 22–30 | Apparent chaos | LOOKS LIKE / CHAOS… | Polyrhythmic shimmer |
| 30–35 | 2 opposite halves (30 s) | HALFTIME: / 2 TEAMS | Call and answer in eighths |
| 35–40 | Chaos again; group-counter HUD slides up | IT'S REWINDING | Bass + pad enter |
| 40–45 | 3 groups (40 s) | COUNT THE GROUPS | Triplets with the groove |
| 45–50 | 4 (45 s), 5 (48 s), 6 (50 s) | 4… 5… 6… | Subdivisions accelerate |
| 50–56 | Groups blur toward one line | TRY IT: 15 WASHERS / + STRING | Build |
| 56–60 | Climax: all 15 snap back in line at 60.0 s = frame 0 | (loop) | One unison chord |

**Flags:**
- Small swings only: period changes slightly with swing size (why real builds drift). Keep the amplitude
  modest on screen; the render can use exact analytic motion so the loop is perfect.
- The cm lengths assume ideal simple pendulums (calc). A real washer-on-string build needs tuning by ear, so
  keep "try it" captions general, not exact cm.
- Overlap with #1 Metronomes: differentiate with setting, colours and the opposite idea (metronomes sync;
  these drift apart and come back).
- Don't copy the famous Harvard video's look (white balls on black): ours is lanterns in a sunlit attic.
- Note density: 15 voices at 51–65 plucks/min ≈ 14 plucks/s. Keep plucks short; trigger once per full swing.

**Feasibility:** very easy: spheres, strings and a frame with θ = θ₀·cos(2π·n·t/60), n = 51…65; no simulation.
One list of trigger times drives both the flashes and the plucks (as on #2). Framing: a portrait frame only
shows ~0.43 × distance in width, so run the row toward the camera at an oblique angle (the iconic view, and
the snake reads best). Floor seen from above under the window light gives free shadow drama.

#### 11 — v2 rework: 100 pendulums (APPROVED, supersedes the v1 beat sheet above)
**User feedback on v1:** increase the scale of the concept: more pendulums. Engagement brief: constantly
stimulating; open on chaos with sound from frame 0; never go quiet; build to a satisfying lock-in; exaggerate.
**Audio check of v1 (rendered audio.py, 2 s windows):** never quiet (−17 → −13 dBFS, no dips), but it opens on
the calmest image (all 15 in a line), real chaos only arrives at ~22 s, and the music builds just ~2 dB.

- **100 pendulums, same rule:** longest swings 51 times a minute, each next one once more, so 51 → 150 swings;
  lengths 34.4 cm → 4.0 cm (calc), a dramatic staircase. All realign at exactly 60 s, so the loop holds.
  Why not 1,000: adjacent pendulums must differ by exactly 1 swing per minute, so the shortest would swing
  ~1,050 times a minute and be under 1 mm long. ~100 is the realistic maximum.
- **What 100 buys:** crisp patterns up to ~20–30 groups (15 pendulums turn to mush past ~6), with interleaved
  kaleidoscope waves between them. One pluck per swing ≈ 167 plucks/s (calc): a real wash of chaos. When
  pendulums bunch into groups their plucks land together and the wash snaps into a rhythm (emergent, from the
  physics). The line-up is a 100-voice strum.
- **Phase offset:** video time = physics time − 8 s. The video opens mid-kaleidoscope, the 6 → 5 → 4 → 3
  group ladder arrives every 2–5 s, patterns accelerate in the back half (~10 → 50 groups between 46 and 51 s),
  and all 100 snap into one line at 52 s (climax), then peel back into the opening. Frame 1800 = physics 68 s
  = physics 8 s = frame 0, so the loop stays exact.
- **Scene:** the ~2.5 m row needs a longer room: stretch the attic into a long gallery with a row of round
  windows, still at sunset. Rainbow-ramp lantern-bobs, longest at the back (staircase silhouette reads); faint
  light trails behind each bob so the waves read as ribbons; floor shadows from above.
- **Audio:** plucks as a granular wash, each pendulum on a pentatonic pitch so the wash stays consonant;
  clusters get louder naturally when pendulums align; build ~+8 dB from the opening to the 52 s strum (v1 built
  ~2 dB).

**v2 beat sheet (60 s; video = physics − 8 s; group times calc):**
| Time | Pendulums | Caption | Sound |
|---|---|---|---|
| 0–2 | Cold open mid-kaleidoscope: 100 bobs, wave-on-wave | 100 PENDULUMS / 1 PERFECT LOOP | Pluck wash (~167/s) + bass hit |
| 2–4 | 6 groups (2 s) | NO MOTOR / NO MAGNETS | Wash snaps into sextuplets |
| 4–7 | 5 groups (4 s); length ramp labels | 34 CM → 4 CM | Quintuplets |
| 7–12 | 4 groups (7 s); camera glides along the row | 51 → 150 SWINGS / PER MINUTE | Sixteenths; filtered groove under |
| 12–17 | 3 groups (12 s) | 3 GROUPS / = TRIPLETS | Triplets |
| 17–22 | Dissolves into chaos | LOOKS LIKE / CHAOS… | Full wash, low-passed groove |
| 22–27 | 2 opposite halves (22 s) | HALFTIME: / 2 TEAMS | Call and answer in eighths |
| 27–32 | Chaos again; group-counter HUD slides up | IT'S REWINDING | Bass + pad enter |
| 32–42 | 3 (32 s), 4 (37 s), 5 (40 s), 6 (42 s) | COUNT THE GROUPS | Groove builds; each pattern a chord change |
| 42–48 | 8 (44.5 s), 10 (46 s), 15 (48 s); patterns accelerate | 10… 15… | Rising build, kick doubles |
| 48–52 | 20 → 30 → 50 groups, converging | (counter racing) | Riser; wash tightens |
| 52 | Climax: all 100 snap into one line | 1 LINE | 100-voice strum on the drop (loudest moment) |
| 52–60 | Peels back into snake and kaleidoscope, loops to the opening | TRY IT: 15 WASHERS / + STRING | Groove rides out into the opening wash |

**v2 flags:**
- Idealised build: a real 100-pendulum rig would need tuning to fractions of a millimetre and drifts with large
  swings. Keep swings small on screen; the render uses exact motion, so don't claim a real build. "Try it" stays
  the 15-washer version.
- Pentatonic pitches are a stylisation: rhythm and timing come from the physics, the notes are a music choice.
- Focus: a ~2.5 m row receding from the camera needs a deep sharp band so all 100 bobs stay crisp (v1's
  focus_check.py).
- Plucks: at ~167/s keep them very short and individually quiet, so the aligned clusters and the strum carry
  the punch.

**v2 feasibility:** easy: same θ = θ₀·cos(2π·n·t/60) for n = 51…150, no simulation, render cost barely changes.
A re-cut in projects/perfect_loop/: keep v1's attic assets, stretch the room into a gallery, rewrite timeline.py
and audio.py (8 s phase offset, granular plucks, the build). Keep a `*_backup.py` of v1 before changing anything.

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_017rbUTK (v1); session_01Mnr8yi (v2, 2026-10-21) · Output: v2: `perfect_loop_v2.mp4` (v1: `perfect_loop.mp4`)
- v1 feedback: scale the concept up (more pendulums). v2 delivered with caption + tags (early sheet sent first): 100 pendulums (51→150 swings, 34→4 cm) in a long gallery with four round windows, physics = video + 8 s so it opens mid-pattern, every group milestone exact and on the 120 BPM grid, HUD counter races 8→100 and snaps to 1 at the 52 s line-up (100-voice strum, loudest moment), build +7 dB (−18 → −11 dBFS), never below the opening; v1 kept as `*_backup.py`. Waiting for feedback
