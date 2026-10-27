# 44. BALLFIGHT: Electric vs Acoustic

Board row: `docs/IDEAS.md` (#44). Series rules: `docs/ballfight/RULES.md`.

## 44. BALLFIGHT: Electric vs Acoustic — APPROVED
**Direction (user, Oct 27):** from the Electric Guitar ideas list the user chose **A. LIGHTNING RIFF**; "Draft Electric
vs Acoustic"; approved with mood B (minor stadium rock). All BALLFIGHT series rules (VS title card without an episode
tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary),
the crossover rules for the ACOUSTIC side (exactly as built in #33; the only per-episode lever is a per-fighter damage
handicap) and the PLAYBOOK's consistent loudness from frame 0 (`reelkit.levels` must PASS).
**Rivalry:** the guitar family feud, plugged vs unplugged ("real guitarists play acoustic" vs "turn it up"). The
ELECTRIC's debut (a new fighter replacing the retired generic GUITAR). Post caption: "Plugged or unplugged? Pick a side
🎸⚡ Comment the next match-up 👇"

| | ELECTRIC (neon green, left), new | ACOUSTIC (wood tan, right), from #33 |
|---|---|---|
| Signature | **LIGHTNING RIFF**: every bar it plays a riff; each riff note is a lightning bolt from its headstock to the enemy, and bolts can also **jump from the ring's corner posts** (lightning rods): the enemy is safe only away from both the electric and the posts; the ropes buzz with sparks (new kind `lightning`) | SIX-STRING RAKE (unchanged): a giant strum sweeps six glowing string lines across the ring, alternating down / up, claw marks |
| Grows per landed hit | +1 bolt per riff | Rake length |
| LV2 | DISTORTION: bolts fork into two branches | CAPO (strums twice as fast) |
| LV3 | TAPPING: a two-handed tapping solo fires bolts on every 8th for a bar | FINGERPICK (single strings flick out like whips) |
| Below 25 HP | GUITAR SMASH: it smashes itself on the floor (huge shockwave + splinter burst), then the pieces snap back together and it keeps fighting (new `smash`) | BIG STRUM (all six strings sweep the whole ring) |

- **Likely shape (only if the honest fight gives it):** the electric controls the corners (posts + itself), so the
  acoustic fights from mid-ring where its rake reaches furthest; GUITAR SMASH vs BIG STRUM would be a huge finish.
- **Colour (in the table):** ELECTRIC neon green (100, 255, 120); lightning = white-hot core + neon-green glow. Strong
  contrast with the acoustic's wood tan (sits between TURNTABLES lime and BAGPIPES green, neither in this fight).
- **Sides:** ELECTRIC left, ACOUSTIC right (the acoustic's usual side).
- **Music, mood B (minor stadium rock), plugged / unplugged:** one original minor arena-rock song (huge drums, big
  room) in two arrangements that follow the leader: when the **electric leads**, the band is plugged in (distorted
  rhythm guitar bed, driving bass, stadium drums); when the **acoustic leads**, the same song goes **unplugged**
  (acoustic strummed bed, upright-ish bass, hand percussion + the stadium kick kept). Crossfade over ~1 beat at each
  lead change; **both versions mastered to the same loudness** (consistent-loudness rule). The **electric plays the
  riffs** (its bolts = riff notes), the **acoustic the strummed chords and picked lines** (its rake strokes = strums,
  FINGERPICK = picked notes); the bed's own guitar never masks either fighter (band-level check). The winner's solo:
  shredded (electric) or fingerpicked (acoustic). `loop_bpm(126)` = 125.90 BPM (32 bars; bar = 1.906 s, FIGHT! at
  3.81 s). (Not chosen: A dark grunge; C desert stoner rock.)
- **VS title card:** ELECTRIC / VS / ACOUSTIC (no episode tag): a distorted power-chord blast with a crackle of
  lightning, the thunder crack, an open-chord strum; panel patterns: pickups + lightning (neon-lit) vs wood grain +
  six strings (reuse #33's).
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.81 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s GUITAR SMASH or BIG STRUM · 44–52 s final exchange · K.O. ~49–53 s
  (slow-mo shatter into pickups / strings / body) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly,
  doors shut, title slam = frame 0.
- **Flags:**
  - The ELECTRIC is a new roster entry (`ELECTRIC`), reusing the existing Pixel Pitch electric-guitar model
    (`electric60_lowpoly`) and its voice (bridge entry `GUITAR`; add an `ELECTRIC` alias if needed, own commit). Its
    colour, lightning and amp sparks keep it visibly different from the retired red GUITAR; the GUITAR entry stays
    untouched (#27 must replay).
  - The ACOUSTIC comes exactly as built in #33; balance with the per-fighter handicap (40–60 % wins).
  - New engine kinds additive, own commits; all earlier episodes replay identically. Bolts: thin, jagged, local;
    no full-screen flashes; `reelkit.flashcheck` must pass (count the bolt bursts per second). GUITAR SMASH's reassembly
    is quick (~1 beat) and it keeps fighting.
  - Honest seed search with `t_fight` = 2 bars and the usual hard windows. Effects from `reelkit.fx`; safe-zone, flash,
    loop and levels checks must pass.
- **Effort:** ~50–65 min (no new model; the lightning system, tuning + seed search, the plugged / unplugged music).
  Work in `projects/ballfight_electric_vs_acoustic/`.

## Board history (moved here from the status board, Oct 27)

- Status: idea (approved, mood B) · Session: — · Output: —
- Approved Oct 27, mood B (minor stadium rock, plugged / unplugged). New ELECTRIC (LIGHTNING RIFF) vs the #33 ACOUSTIC. No episode number on screen. Ready for production.
