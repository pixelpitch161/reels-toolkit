# 16. The Metronome Choir (100 Metronomes, part 2)

Board row: `docs/IDEAS.md` (#16). Series rules: none.

## 16. The Metronome Choir (100 Metronomes, part 2) — APPROVED
**Direction (user, Oct 22):** the science videos are cool but lack #1's allure. Focus on ideas that are simply
cool and satisfying to watch and listen to; science is optional flavour. Keep the engagement brief: chaos and
sound from frame 0, never quiet, build to a satisfying lock-in, exaggerate freely.
**Why a sequel:** #1's formula (random ticking → board onto 2 soda cans → crowd pulls everyone in → one
holdout spotlighted → drop at ~38 s with an LED tip show) is kept beat for beat and multiplied: 5 sections lock
in one by one, each adding a layer of harmony, then merge into one choir; the holdout becomes the soloist whose
lock-in brings the melody.
- **Hook:** "200 METRONOMES" / "= 1 SONG" (14 + 8 chars, in the first 2 s). Alt: "PART 2:" / "200 METRONOMES".
- **What you see:** 200 metronomes on 5 tiered risers like a choir loft (40 per riser; each riser a long
  board; each section its own colour), plus a golden soloist metronome on a conductor's podium in front. Tip
  colours show phase as in #1 (mint = in sync, coral = way off).
- **How it plays:** every tick rings a bell. Each section has a two-note pair (one bell on the left swing, one
  on the right); the 5 sections stack into a chord vamp (bass, tenor, alto, soprano, high bells). The soloist
  carries a small bell carousel that steps to the next melody note on every tick. Unsynced = a chaotic
  bell-tick wash (~400 ticks/s). As each riser goes onto soda cans and syncs, its two-note line rises clean out
  of the wash (like a stem fading in). Then all 5 risers go onto one giant shared platform: sections that are
  each in sync but out of step with each other pull together into one pulse. The soloist drifts against the
  beat until the final lock-in = the drop: an original arrangement at 120 BPM (bass, drums, chords) locked to
  the ticks, with the soloist's melody on top.
- **Scene:** low-poly concert hall in the afternoon: big round skylight (sun = key light), warm stage spots
  (local lights), dust in the beams; small life: a cat on the grand piano, a few pawn-figure listeners in the
  front row. Callbacks to #1: the same soda cans (giant ones under the shared platform) and the LED tip show
  at the drop.
- **Ending / loop:** the song rides out; someone kicks the cans out, the platform thumps down and the jolt
  throws every metronome back into the random chaos of the opening.

**Beat sheet (60 s; never quiet, a board placement or lock-in every 4–5 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–3 | Cold open: all 200 ticking at random on fixed risers | 200 METRONOMES / = 1 SONG | Chaotic bell-tick wash |
| 3–6 | Camera sweeps the 5 colour-coded sections | 5 SECTIONS / 5 VOICES | Wash; low dissonant pad |
| 6–9 | Bass riser lifted onto soda cans (clunk) | BASS → / SODA CANS | Clunk, rolling rumble |
| 9–13 | Bass tips turn mint row by row | BASS: LOCKED | Two-note bass line rises out of the wash |
| 13–17 | Tenor riser onto cans; locks in | TENOR: LOCKED | Second voice joins |
| 17–21 | Alto + soprano risers together | 4 OF 5 | Chord fills in; wash thins |
| 21–25 | High-bell riser locks; the 5 sections tick out of step with each other | EACH IN SYNC… / NOT WITH EACH OTHER | Clean voices, flammed against each other |
| 25–30 | All 5 risers hoisted onto one huge platform on giant cans | NOW: / ONE BIG BOARD | Big clunk, sway rumble |
| 30–34 | Sections drift together; flams tighten; counter climbs | NOBODY IS / IN CHARGE | Flams → unison; kick enters on the pulse |
| 34–38 | 199 locked; spotlight on the coral soloist swinging off-beat | 199 IN SYNC… / ONE SOLOIST | Melody notes drag against the beat; riser + snare roll |
| 38–40 | Soloist locks in | 200/200 | Drop |
| 40–52 | LED tip show, camera orbits, melody on the grid | THE BOARD IS / THE CONDUCTOR | Full song: bells + bass + drums + melody |
| 52–58 | Wide shot of the whole choir; song rides out | TRY IT: 2 METRONOMES / + 2 SODA CANS | Final chorus |
| 58–60 | Cans kicked out, platform thumps, everyone jolted back to random; loop | (no caption) | Thump into the opening wash |

**Flags:**
- Physics is real, the mechanisms are invented. Real: metronomes on a free-moving board sync, and boards on
  one shared platform can pull each other into sync (same physics as #1). Invented: the two-note bells and the
  soloist's melody carousel. Fine for the satisfying format; don't caption them as science.
- The jolt reset is staged (removing the coupling alone wouldn't instantly scramble them; the thump does). It
  reads as a comic beat, not a claim.
- Tick density ~400/s in the cold open: keep individual ticks short and quiet so each section's clean line
  clearly pops out when it locks.
- Music: original arrangement at 120 BPM (the metronomes' tempo, as in #1), in a different key and feel from
  #1's A-minor chiptune so the sequel doesn't sound like a repeat.
- Branding: keep #1's low-poly look, metronome model and tip colours so returning viewers recognise the
  series; the hall setting is new.

**Feasibility:** reuse `projects/100_metronomes/` (sim.py's Kuramoto model, metronome + soda-can models, tick
sounds, LED tip show). Extend the sim to two layers: coupling inside a board switches on when that riser goes
onto cans; a global coupling switches on with the shared platform. Choose the soloist's frequency offset so it
locks last at ~38 s (as #1's holdout); time the carousel so the melody starts on note 1 at the drop. New: 5
risers, the podium and the hall; 200 instanced metronomes (2× #1) is cheap. Audio: section bells as two pitched
tick voices per section, panned by riser position; new drop arrangement. A tall tiered choir loft suits
portrait; push in on the soloist for 34–38 s. Work in a new `projects/metronome_choir/` (copy from #1, don't
edit #1's files).

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01EvbHha (2026-10-22) · Output: `metronome_choir.mp4`
- v1 delivered with caption + tags (early sheet sent first). Beat sheet followed; one change: the 3-6 s sweep became a roll call (each section lights up in turn) because the fast sweep over the bright/dark tiers tripped the flash test. Sim locks: bass 9.6, tenor 15.9, alto+soprano 20.0, high 23.5, choir 33.1, soloist 37.8 s (drop tick 1-3 ms off the grid). Checks: focus CoC 0 on all frames; flash test worst 1/s (7 isolated, camera moves/cuts); bells 7-26 dB over the music; -13.6 LUFS, opening -18.5 dBFS -> drop -13.3; loop seam 4.2 vs 2.3-3.3. Waiting for feedback
