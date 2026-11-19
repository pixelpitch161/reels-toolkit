# PICCOLO fighter: design

Aedan asked (2026-11-16) for a new fighter, PICCOLO, made from scratch (no Python kit reference: Rust is the source of
truth). Three move sets were drafted with visual concepts (https://claude.ai/artifact/8Pk2RFFykppcq5RkYcXGaH) and
Aedan picked **A, Sharpshooter**. Process rules: once the design is approved, build it through to a merge into the
build branch without stopping; do not install; a lenient balance check (fights auto-balance by handicap and seed).

## Approaches considered

- **Sharpshooter** (picked): a thin, very fast note that ricochets off the ropes and pierces. MARIMBA (merged the same
  night) also ricochets, so PICCOLO's needle is kept visibly different: it moves about five times faster and is drawn
  as a streak of light that traces its whole zig-zag, it pierces through the enemy instead of being spent, and its
  damage does not climb per bounce. What grows with landed hits is how many bounces it has.
- Pipsqueak: the ball itself dashes and pecks on the beat.
- Songbird: an orbiting flock that dives; closest to FLUTE's summoned cobras.

## The fighter

- Name `PICCOLO`, colour **Lemon** rgb(248, 240, 144) (dark rgb(150, 140, 60)): the colour furthest (CIE Lab) from
  every roster fighter, the sibling fighters' colours and the reserved KAZOO red and HARMONICA sky blue, among the
  bright ones that glow on a dark ring. Bright and pale, like the highest note in the band.
- Model: built on Aedan's PC from real piccolo photos (Aedan's rule: every new instrument uses web photo references),
  `bf-content/data/models/PICCOLO.json`, generator `studio/scripts/models/piccolo_model.py`, notes and photos in
  `studio/reference/models/piccolo/`. A black grenadilla body with silver keys.
- Usually plays the melody, the highest register in the roster (about D5 to C8); idiom `needle`. Section: woodwinds.

## Weapon: PIERCING NOTE (new kind `needle`)

On its grid (`every` beats, haste divides it) the piccolo fires one needle at the enemy as a bank shot (mirrored in
one rope, the ropes in turn, leading the enemy's motion). A needle flies straight at `speed` (very fast) and bounces
off the ropes; after `int(n_vol)` bounces (`n0` = 1 at the start, `grow` per landed hit, up to `n_max` = 5) it
fizzles at the next rope. It hits the enemy it passes through (within `hit_r` + the ball's radius) for `dmg` and a
knock along its flight, once per needle, and **keeps flying** (it pierces). It passes through its owner. Events
`needle` (fire), `needle_bounce`, `needle_hit`, `needle_end`; hits of kind `needle`, the bounce count as the
sub-number; recorded per frame as `NEEDLES`.

## Abilities

- **TRILL** (unlock 1, the existing kind `haste`): the grid runs twice as fast for 8 beats; the rope rotation makes
  the shots alternate between angles like a trill.
- **OVER THE BAND** (unlock 2, new kind `over_band`): for `dur_beats` beats its needles cut through shields (a
  shielded enemy is hit as if it had none; event `pierce`).
- **GLASS SHATTER** (below 25 HP, new kind `shatter`): `n` glass shards break off the ropes, spread evenly round the
  ring, one after another over `beats` beats, each flying at the enemy's position at that moment at `speed`; a shard
  hits once for `dmg` and is gone, or breaks at the far rope. Hits of kind `shard`.

## Look and sound

- Look: the model as the ball's sprite; each needle drawn as a bright streak along its last stretch of path (the
  path folded at every bounce), a white-hot head, a spark on each bounce and on each pierce; OVER THE BAND gives the
  streaks a wider glow; shards as small pale triangles in its colour; glass splinters on a K.O.
- Sound: a piccolo voice (a bright, nearly pure tone with breath and a quick attack, a little vibrato on long
  notes); each shot plays its line's note with a grace note, each bounce a quick high tick up the scale, a pierce
  a bright chord tone; TRILL plays trills; OVER THE BAND a held shrill top note; GLASS SHATTER a falling shower of
  glassy high notes. Personality motifs are invented for it (fife-march figures, trills, octave leaps).

## Calls left to the builder

Numbers (speed, damage, grid) are set by a short lenient balance run against a few opponents. Card text, tags, theme
fit traits and the title-card pattern follow the other fighters' modules.
