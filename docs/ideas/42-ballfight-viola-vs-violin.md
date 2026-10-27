# 42. BALLFIGHT #16: Viola vs Violin

Board row: `docs/IDEAS.md` (#42). Series rules: `docs/ballfight/RULES.md`.

## 42. BALLFIGHT #16: Viola vs Violin — APPROVED
**Direction (user, Oct 27):** from the Viola ideas list the user chose **A. COUNTING RESTS**; "Draft Viola vs Violin";
approved with mood A (orchestral drill); "make sure the session uses chrome to make the model for Viola". All
BALLFIGHT series rules (VS title card, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight,
61.0 s, loop point = file boundary), the crossover rules for the VIOLIN side (exactly as built in #28; the only
per-episode lever is a per-fighter damage handicap) and the PLAYBOOK's consistent loudness from frame 0
(`reelkit.levels` must PASS).
**Rivalry:** the orchestra's oldest joke: the violin gets every melody, the viola gets the rests and the punchlines.
The viola's revenge. Post caption: "Violists, this one's for you 🎻 Comment the next match-up 👇"

| | VIOLA (teal, left), new | VIOLIN (violet, right), from #28 |
|---|---|---|
| Style | The only charger: slow, huge hits | Fast and precise; the bow speeds up with every hit |
| Signature | **COUNTING RESTS**: for 3 beats it rests (a giant quarter-rest symbol over it, small "1 2 3" counting above, half damage taken); on beat 4 a giant **whole note** (the hollow oval) appears and rolls forward like a heavy wheel, sustaining for the whole bar and crushing anything in its path (new kind `rests`: a rest-and-charge weapon + a rolling sustained note) | The bow, spun around the ball like a rapier, faster per landed hit (unchanged) |
| Grows per landed hit | Whole-note size + power | Bow spin speed |
| LV2 | DOUBLE STOP: two whole notes at once (at the violin and where it's heading) | PIZZICATO (fan of 5 plucked darts) |
| LV3 | MELODY AT LAST: rests shrink to 1 beat (a whole note every other beat) for 2 bars, and the music hands the viola the melody | TREMOLO (bow blurs at 2.5x for 3 s) |
| Below 25 HP | VIOLA JOKE: a rimshot and the whole crowd bursts out laughing; the violin freezes for a beat (new `stun`-style rule, or reuse #37's `stun` if it fits), then the biggest whole note lands. No joke text on screen, just the laugh (crowd silhouettes rocking) | SCREECH (ring-wide squeal wave) |

- **Likely shape (only if the honest fight gives it):** speed vs patience. The violin's chip damage is halved while the
  viola rests, but the violin keeps getting faster; every whole note is slow, telegraphed and huge. MELODY AT LAST is
  the story beat: the viola finally gets the melody, in the music and in the fight.
- **Colours:** VIOLA teal (36, 200, 176) (freed by the retired SAX) vs VIOLIN violet.
- **Sides:** first named = left: VIOLA left, VIOLIN right (the violin's usual side).
- **Music, mood A (orchestral drill):** a UK drill beat (sliding 808s, skittering hi-hat rolls, half-time snare) under a
  dark staccato string bed (cellos + basses only; no violin or viola in the bed). The **violin plays the melody**, the
  **viola the inner harmony** (warm, a fifth lower), **until MELODY AT LAST, when the arrangement flips and the viola
  leads** for its 2 bars. Whole notes = long sustained viola notes over the beat; the counting rests are audible as a
  soft muted pluck on 1-2-3. `loop_bpm(140)` = 141.64 BPM (36 bars; bar = 1.694 s, FIGHT! at 3.39 s). Consistent loudness
  from frame 0 (the drill beat full on the title card; build with density). VIOLA JOKE: a rimshot (ba-dum-tss) + crowd
  laugh, then the big note. Leader fuller and louder, loser thinner and dirtier; the winner's solo over the drill beat.
  (Not chosen: B tango noir; C dark synthwave.)
- **VS title card:** VIOLA / VS / VIOLIN: a dark viola chord, the thunder crack, a violin flourish; panel patterns: alto
  clef + f-holes (teal-lit) vs violin scroll + f-holes (violet-lit; reuse #28's).
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.39 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s VIOLA JOKE or SCREECH · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter into scroll / strings / bridge) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors
  shut, title slam = frame 0.
- **Flags:**
  - **The VIOLA model is built from reference photos found with Claude in Chrome** (user's instruction), as the trumpet,
    trombone, oboe and bassoon were: look up front / side / 3/4 photos of a real viola (e.g. Wikimedia Commons), measure
    proportions (body ~40–42 cm vs the violin's 35.5 cm, deeper ribs, wider bouts, darker varnish), and build a new file
    in `series/instrument-history/` (own commit). The violin model's structure can be a starting point, but the shape
    comes from the photos, and the size difference must show on screen. Voice: a darker bowed voice a fifth below the
    violin (C string), own commit; bridge entry.
  - The VIOLIN comes exactly as built in #28; only the optional visual upgrade (rosin-dust ribbon behind the bow,
    BALLFIGHT v2 effects kit) is allowed; #28 must replay identically. Balance: tune with the per-fighter handicap
    (40–60 % wins).
  - New engine kinds additive, own commits; all earlier episodes replay identically. Honest seed search with
    `t_fight` = 2 bars and the usual hard windows (keep "no 4 s without a hit": the viola's rests must not stall the
    fight). Counting numbers are small and inside the ring (not captions). Effects from `reelkit.fx`; no full-screen
    flashes; safe-zone, flash, loop and levels checks must pass.
- **Effort:** ~55–70 min (one new model from photos + voice, a new weapon type). Work in
  `projects/ballfight_viola_vs_violin/`.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01CzTfqcW39D (2026-10-27) · Output: ballfight_viola_vs_violin.mp4
- Delivered Oct 27 for review: new VIOLA (model from Chrome reference photos, x1.12 + deeper ribs; voice a fifth below) left vs #28 VIOLIN right, engine kinds rests / double_stop / melody / joke, handicap 0.75 / 2.1, honest seed 8219 (VIOLA wins on 0.9 HP at 50.41 s, VIOLA JOKE at 41.5 s), mood A orchestral drill. Feedback: (pending)
