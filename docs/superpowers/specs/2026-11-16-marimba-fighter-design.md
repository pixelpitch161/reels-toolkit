# MARIMBA fighter: design

Aedan asked (2026-11-16) for a new fighter, MARIMBA, made from scratch (no Python kit reference: Rust is the source of
truth). Three move sets were drafted with visual concepts (https://claude.ai/artifact/MGjXuZfP91kh2n9XDG4WPH) and
Aedan picked **Mallet Run**. Process rules: once the design is approved, build it through to a merge into the build
branch without stopping; do not install; a lenient balance check (fights auto-balance by handicap and seed).

## Approaches considered

- **Mallet Run** (picked): ricochet mallets that climb the scale on every wall bounce. Reads fastest on a phone and its
  music and its fight are the same thing.
- Rosewood Bars: bars laid in the ring as bumpers that ripple. Crowds the ring beside HARP's strings.
- Phase Ghosts: out-of-phase echo copies that strike late. Close to BASS CLARINET's MULTIPHONICS twin, harder to read.

## The fighter

- Name `MARIMBA`, colour **Jacaranda** rgb(160, 80, 160) (dark rgb(94, 44, 94)): the colour furthest (CIE Lab) from
  every roster fighter and from the reds the sibling fighters took (oxblood, mahogany, scarlet, cherry, rose brass).
- Model: built on Aedan's PC from real marimba photos (Aedan's rule: every new instrument uses web photo references),
  `bf-content/data/models/MARIMBA.json`, generator `studio/scripts/models/marimba_model.py`, notes and photos in
  `studio/reference/models/marimba/`.
- Usually plays the melody (register about C4 to C7); idiom `mallets`.

## Weapon: MALLET RUN (new kind `mallets`)

On its grid (`every` beats, haste divides it) the marimba throws a mallet at the enemy while fewer than `int(n_vol)`
of its mallets are in the air (`n0` = 1 at the start, `grow` = 0.25 per landed hit, up to `n_max` = 4: the four-mallet
grip). A mallet flies straight at `speed` and bounces off the ropes; every wall bounce is the next note up the scale
and adds `dmg_step` to its damage. It hits the enemy when it touches the ball (`dmg` + `dmg_step` x bounces, a knock
along its flight) and is gone; it passes through its owner; after `bounces` wall bounces it fizzles. Events
`mallet` (throw), `mallet_bounce`, `mallet_end`; hits of kind `mallet` with the bounce count as the sub-number;
recorded per frame as `MALLETS`.

## Abilities (new kinds, in the weapon's module)

- **TREMOLO** (`tremolo`, unlock 1): for `dur_beats` beats every mallet in the air buzzes as a pair and a hit lands
  twice (a second hit of kind `tremolo` worth `mult` of the first).
- **DEAD STROKE** (`dead_stroke`, unlock 2): arms the marimba for `dur_beats` beats; the next mallet to land pins the
  enemy in place for `pin_beats` beats (the engine's stun: it holds still and starts no attack). Event `dead_stroke`.
- **FIVE OCTAVES** (`five_octaves`, below 25 HP): `n` = 61 mallets leave the ball one after another over `beats` beats
  in a full rising turn starting at the enemy, each bouncing at most once, `dmg` each.

## Look and sound

- Look: the model as the ball's sprite; mallets drawn over the ring (a pale birch shaft and a yarn head in its colour
  that glows hotter with every bounce; pairs under TREMOLO), a bright tick on each wall bounce, a dry burst for DEAD
  STROKE, mallet-head and bar shards.
- Sound: a marimba voice (fundamental with the 3.93 and 9.2 partials and a mallet click); every throw plays its line's
  note, every bounce the next scale tone up from it, a landed hit a chord tone; TREMOLO a rolled tremolo; DEAD STROKE a
  muted thunk; FIVE OCTAVES a full rising glissando. Personality motifs are invented for it.

## Calls left to the builder

Numbers (speed, damage, grid) are set by a short lenient balance run against a few opponents. Card text, tags, theme
fit traits and the title-card pattern follow the other fighters' modules.
