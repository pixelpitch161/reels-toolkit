# 41. BALLFIGHT #15: Flute vs Trombone

Board row: `docs/IDEAS.md` (#41). Series rules: `docs/ballfight/RULES.md`.

## 41. BALLFIGHT #15: Flute vs Trombone — APPROVED
**Direction (user, Oct 27):** "Draft Flute vs Trombone"; approved with mood A (sinister big top). A crossover: FLUTE as
built in #32 v2, TROMBONE as built in #34. All BALLFIGHT series rules (VS title card, 3·2·1, ability-name captions, HUD
ability list, colours, honest fight, 61.0 s, loop point = file boundary), the **crossover rules** (#36: fighters exactly
as in `fighters.py`; the only per-episode lever is a per-fighter damage handicap tuned to 40–60 % wins; flag instead of
changing a fighter; all earlier episodes replay identically) and the PLAYBOOK's consistent loudness from frame 0
(`reelkit.levels` must PASS).
**Rivalry:** the front row vs the back row: a real band-room feud (flutes sit up front, trombones behind them poke them
with their slides); now the flute answers. Post caption: "Band kids know 😤 Front row or back row? Comment the next
match-up 👇"

| | FLUTE (silver, left), from #32 v2 | TROMBONE (mint, right), from #34 |
|---|---|---|
| Signature | SNAKE CHARMER: drops baskets (up to 2); silver cobras rise, sway and strike on the beat | SLIDE LANCE: aims and shoots the slide out on every beat; swats things it passes through |
| Grows per landed hit | Cobra strike reach + speed | Lance reach |
| LV2 | FLUTTER TONGUE (cobras strike twice as fast) | SAD TROMBONE (shockwave + 1 s slow, "wah wah wah waaah") |
| LV3 | SNAKE DANCE (cobras leave the baskets and chase for a bar) | POWER SLIDE (thrusts on 8ths) |
| Below 25 HP | PICCOLO (piercing ring-wide note, glass-crack marks) | PEDAL TONE (a giant bell drops and stomps) |

- **Likely shape (only if the honest fight gives it):** the trombone wants to poke from range (the back-row joke made
  real); the flute wants it close, into its baskets. Cobras can be swatted if the lance passes through them; SNAKE DANCE
  sends them hunting; SAD TROMBONE's slow can pin the flute in a cobra-free spot or drag the trombone into one. Comedy
  finish possible: PICCOLO vs PEDAL TONE, highest note vs lowest.
- **Sides:** first named = left: FLUTE left, TROMBONE right, both swapped from their own episodes. Set the sides for
  this episode only (as #40 did; don't edit the roster's `corner` values).
- **Colours:** silver vs mint are both pale. Check on the early sheet that they read apart (cool white vs green); if
  not, add a stronger edge glow to each ball, never a colour change.
- **Lance vs cobras:** if the engine doesn't define whether a thrust swats a cobra, add it as an opt-in rule (own commit)
  and tell the user, rather than changing either fighter.
- **Music, mood A (sinister big top):** a dark circus: calliope / band-organ, oom-pah bass, minor carnival chords,
  snare rolls and cymbal crashes; the cobras are the sideshow and SAD TROMBONE is the clown. All original (no quoting
  circus marches). The **trombone plays the bass and smears**, the **flute the high melody** (the widest register gap in
  the series); the bed has no brass or flute of its own. `loop_bpm(132)` = 133.77 BPM (34 bars; bar = 1.794 s, FIGHT! at
  3.59 s). Consistent loudness from frame 0 (full band on the title card; build with density). Leader fuller and
  louder, loser thinner and dirtier; the winner's solo over the big top. (Not chosen: B dark synthwave; C minor
  afro-house.)
- **Scene touch (optional, cheap):** circus-tent stripes in the arena's backdrop lighting or pennants; keep the ring
  and HUD identical to the series.
- **VS title card:** FLUTE / VS / TROMBONE: a flute flurry, the thunder crack, a trombone smear; existing panel patterns
  from #32 / #34.
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.59 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s PICCOLO or PEDAL TONE · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors shut, title slam = frame 0.
- **Flags:** honest seed search with `t_fight` = 2 bars and the usual hard windows. Effects from `reelkit.fx`; no
  full-screen flashes; safe-zone, flash, loop and levels checks must pass.
- **Effort:** low (~40–50 min): no new attacks, models or voices; the side swap, tuning + seed search, the new music.
  Work in `projects/ballfight_flute_vs_trombone/`.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01CzTfqb (2026-10-27) · Output: `ballfight_flute_vs_trombone.mp4`
- v1 delivered with caption + tags (early sheet sent first). Crossover: FLUTE #32 v2 (left) vs TROMBONE #34 (right), sides set in the timeline only. Lance vs cobras was undefined: new opt-in rule `Rules(swat_cobras=True)` (own commit b346575; told the user): a live lance passing through a risen cobra knocks it flat (a strike in the air misses, no strikes for 2 beats; no damage, no growth); all 15 finished episodes replay identically. Handicap FLUTE x2.0 / TROMBONE x0.40 (300 seeds 47 / 53 %, median K.O. 49.6 s). Silver vs mint read apart on the early sheet (cool steel glass vs green): no edge glow. Honest search at 2 bars: 8 fits in 968 (49 / 51 %); seed **392**: even to 35 s, lead changes 22.5 / 42.9 s, PICCOLO 42.6 s answered by PEDAL TONE 43.1 s (misses), 23 swats, TROMBONE wins at 50.02 s on 4.1 HP. Captions FLUTTER TONGUE 11.2, SAD TROMBONE 16.2, POWER SLIDE 26.0, SNAKE DANCE 30.1, PICCOLO 42.6 (PEDAL TONE in the HUD list: 4 s rule). Tail: the glides home crossed (the flute fell on the trombone's side), so both bow apart (1.41 at the closest). Music mood A (E harmonic minor, 133.77 BPM; calliope off-beats, bass drum, choked cymbal, no brass / flute in the bed): trombone = the circus bass + smears, flute = the high melody; 3-bar trombone solo. Checks: `reelkit.levels` PASS on wav + mp4 (opening +0.1 dB vs ending, spread 3.6 dB, no dips; one master gain); band trombone +6.6..+7.8 dB, flute +12..+16 dB over the bed; intro -1.5 LU vs the fight; safe zone OK; CoC 0 on all 1830 frames; flash test 0 on 40.5-51.5 s, flashcheck PASS (1 = the doors); loopcheck PASS render 0.4x / mp4 1.1x. Render 10.8 min + mux. Guard entry #41 -> seed 392. Waiting for feedback
