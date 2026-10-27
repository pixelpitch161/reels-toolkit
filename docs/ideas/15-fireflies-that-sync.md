# 15. Fireflies That Sync

Board row: `docs/IDEAS.md` (#15). Series rules: none.

## 15. Fireflies That Sync — APPROVED
**Engagement brief (user, Oct 2026):** constantly stimulating, especially at the start. Open on chaos with
sound from frame 0, never go quiet, build to a satisfying lock-in (the 100 Metronomes arc); exaggerate where the
phenomenon is calmer.
- **Hook:** "1 FLY: RANDOM" / "20 FLIES: SYNC" (13 + 14 chars, over the random sparkle, in the first 2 s).
- **Science:** *Photinus carolinus*, Great Smoky Mountains (Elkmont). Each male flashes 5–8 times, then pauses
  ~8 s in the dark (NPS). Flashes last 100–150 ms, ~0.55 s apart (Sarfati, Hayes & Peleg, *Science Advances*
  2021). Early in the evening flashing looks random; the rhythm aligns as more males join (NPS). A lone male
  flashes with no clear rhythm; synchrony starts to appear at ~20 fireflies, because each watches its
  neighbours and adjusts (Sarfati et al., *J. R. Soc. Interface* 2020). In big swarms each burst rolls across
  the forest as a wave at ~0.5 m/s: the signal moves, not the insects (2021). Females are much dimmer and
  answer in the males' dark pauses (NPS). Why they sync isn't fully understood.
  Sources: [NPS: Synchronous Fireflies](https://www.nps.gov/grsm/learn/nature/fireflies.htm),
  [Sarfati et al. 2021, Science Advances](https://pmc.ncbi.nlm.nih.gov/articles/PMC8262802),
  [CU Boulder (Sarfati et al. 2020)](https://www.colorado.edu/today/synchronous-fireflies).
- **Music:** every male flash = a bell note (each firefly its own pentatonic pitch), so random flashing = a
  chaotic sparkle. As they lock in, the spread between flashes shrinks: smear → flam → unison, until each flash
  is one chord. Tempo from the real flash spacing: ≈109 BPM (calc); one burst = a 2-bar, 8-beat phrase. Each
  dark pause is the build; the females' dim answers are soft low notes (real call and response). Each burst is
  the drop. Burst waves rolling across the forest = arpeggio sweeps panned left to right. Original groove + bass
  hold the dark pauses. All synthesised.
- **Scene:** low-poly Elkmont-style forest: tall tulip poplars, ferns, mossy boulders, a creek. Moon = key
  light (keeps the playbook's lit level); warm local light from distant visitors' red-filtered trail lights
  (nod to the NPS red-light rule). Males drift 1–2 m above the ground; females are dim green points in the leaf
  litter. Escalation: 1 → 20 → hundreds → the whole valley.
- **Payoff / ending:** climax burst: thousands, waves rolling in from both sides and meeting, the whole valley
  flashing on the beat over the full groove; females answer in the pause. The camera returns to a lone male
  flashing at random ("EVERY NIGHT IT STARTS RANDOM"), looping to the opening.

**Beat sheet (60 s; at 109 BPM one cycle = burst of 8 flashes 4.4 s + ≈8.8 s dark (16 beats) = 13.2 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Cold open: forest sparkling with random flashes | 1 FLY: RANDOM / 20 FLIES: SYNC | Random bell sparkle + low pad |
| 4–8 | Push in on one male; irregular bursts; sparkle as bokeh behind | ALONE: / NO RHYTHM | His stray notes over the sparkle wash |
| 8–12 | A few neighbours join; flashes start to cluster | EACH ONE WATCHES / ITS NEIGHBOURS | Notes bunch into flams |
| 12–16 | ~20 converge over a few bursts | AT ~20: / THEY LOCK IN | Smear → flam → unison; pulse emerges |
| 16–21 | More males join; counter ticks 1–8 | 8 FLASHES / 0.5 S APART | Kick + bass enter on the flash beat |
| 21–25 | Burst 1: dozens in unison | (no caption) | Drop 1: chord on every flash |
| 25–34 | Dark pause; dim green answers from the ground; camera pulls back | ≈8 S OF DARK… / FEMALES ANSWER | Soft low answers + groove + riser |
| 34–39 | Burst 2: hundreds; the flash rolls across the screen as a wave | THE FLASH ROLLS / AT ~0.5 M/S | Drop 2: arpeggio sweeps L → R |
| 39–47 | Dark pause; more fireflies rise from the ferns | WHY SYNC? / STILL A MYSTERY | Answers + bigger riser |
| 47–52 | Climax burst 3: whole valley, waves meet in the middle | (no caption) | Full groove + melody, unison chords on every flash |
| 52–56 | Pause; females answer across the valley | SEE IT: SMOKY MTNS / LATE MAY–JUNE | Answers + groove riding out |
| 56–60 | Camera returns to the lone male; random sparkles begin; loop | EVERY NIGHT IT / STARTS RANDOM | Sparkle returns into the opening |

**Flags:**
- Cycle timing varies by source (NPS: 5–8 flashes then ~8 s dark; 2021 study: bursts repeat ~every 12 s).
  Caption "≈8 S"; the 8.8 s pause is a music-friendly choice within that range.
- 109 BPM is calc from the 0.55 s spacing, and rates vary night to night: don't caption a precise BPM.
- Lighting is stylised: the real display is in full darkness; the moonlit look keeps the playbook's lit level,
  with fireflies popping through glow and bloom.
- "Random at first" is the start of the evening: show a modest number of random flashers in the cold open, not
  thousands; reach thousands only once synced.
- Photosensitivity: ~1.8 flashes/s (under 3), but the climax waves cover much of the frame. Keep the brightness
  change per flash moderate and run #13's flash_check.py.
- Responsible viewing: the ending mentions the lottery season; never show catching fireflies or bright lights.
- Overlap with 100 Metronomes is deliberate (another sync story; that one did well). Differentiate with the
  night forest, call and response, and travelling waves.

**Feasibility:** easy: fireflies as instanced glow particles; phases from a simple coupled-oscillator model
(adapt the Kuramoto code in `projects/100_metronomes/sim.py`); waves = phase delay by distance (0.5 m/s);
females = dim ground points. Medium: the forest (low-poly trees, ferns, creek) with moon shadows on the floor
from a slightly high camera. Audio: thousands of flashes per burst, so group them into short time bins and play
each bin as a chord (louder for more flashes, panned by screen x). One flash schedule drives picture and sound.
Bell notes are mid–high and survive phone speakers; keep a click on each unison flash.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_012WAWkH (2026-10-21) · Output: `fireflies_sync.mp4`
- v1 delivered with caption + tags (early sheet sent first). Kuramoto lock-in (274 -> 8 ms spread over burst A), 0.5 m/s wave band, valley climax; first gap shortened to 8 beats so drop 1 lands at 21 s (all later gaps 16 beats). Checks: hero CoC 0 on all frames, 0 general flashes (climax 22 % area), bells 15-34 dB over music. Waiting for feedback
