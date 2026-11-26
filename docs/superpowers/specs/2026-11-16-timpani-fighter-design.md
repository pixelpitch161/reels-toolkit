# TIMPANI fighter: design

Aedan asked (2026-11-16) for a new fighter, TIMPANI, made from scratch (no Python kit reference: Rust is the source of
truth). Three move sets were drafted with visual concepts (https://claude.ai/artifact/YBQ6MsLmnBzpizFEbWxHVW) and
Aedan picked **B, Two Kettles**. Process rules: once the design is picked, build it through to a merge into the build
branch without stopping; do not install; a lenient balance check (fights auto-balance by handicap and seed).

## Approaches considered

- **Two Kettles** (picked): the timpani sets a low and a high kettle in the ring and hops between them; each landing
  booms that kettle. The most orchestral idea, and its music (two tuned drums, roots and fifths) is the fight.
- Pedal Gliss: a ring that goes out, then pulls the enemy back as the pedal bends the pitch up.
- Felt Mallets: overhead mallet strikes that leave humming marks; closest to BASS DRUM's cannons and FLUTE's strikes.

## The fighter

- Name `TIMPANI`, colour **Silver** rgb(178, 184, 196) (dark rgb(92, 98, 112)): a chrome kettle; the only bright
  grey in the roster (BASS DRUM's gunmetal is far darker, FLUTE's white far paler), and clear of the reds and browns
  the sibling fighters took (oxblood, rosewood, scarlet, cherry, rose brass).
- Model: built on Aedan's PC from real timpani photos (Aedan's rule: every new instrument uses web photo
  references; photos and notes in `F:\!Ai Stuff\InstrumentBattleMachine\refs\timpani`): a cream head ringed by a
  chrome hoop with 8 tension rods, a hammered copper bowl, struts and a pedal. `bf-content/data/models/TIMPANI.json`.
- Usually plays the roots and fifths (register about D2 to A3, MIDI 38 to 57); idiom `timpani`. Section: percussion.
- Must stay apart from DRUMS (drumhead pulse, drum roll, crash), BASS DRUM (steamroller, booms, cannons) and SNARE
  DRUM (stickwork): it never rolls or fires a shockwave from its own ball; its rings come from the kettles.

## Weapon: TWO KETTLES (new kind `kettles`)

- Setup: the timpani is a hopper (it stands still and slides only when knocked, like BASSOON). Two kettles are set
  on the floor on a diagonal at `spot` x the ring's half-size: the low one (radius `r_low`) on its own side, the high
  one (`r_high`) across. It starts standing on the low one.
- Grid: every `every` beats (a haste divides it), when not stunned and not in the air, it hops to the next kettle,
  `air16` 16ths in the air (a haste divides that too, at least 1). In the air it cannot clash. It never lands inside
  the enemy (it steps out past it).
- Landing: that kettle booms: a ring leaves the kettle's centre at `speed_low` / `speed_high` out to `reach_low` /
  `reach_high`; the ring hits the enemy once when its front reaches it (`dmg_low` / `dmg_high`, knock away from the
  kettle `knock_low` / `knock_high`). An enemy hit by both kettles' rings within `pair_t` seconds takes the interval
  hit on top (`dmg_pair`, kind `interval`).
- The enemy can shove a kettle: a kettle it overlaps slides out to touch it, kept inside the ropes. The timpani's
  hops go wherever the kettle is when it takes off.
- Events: `kettles` (setup: each kettle's x, y, radius), `kettle_hop` (from, to, landing time), `kettle_boom`
  (kettle, x, y, reach), `kettle_move` is not an event (the record carries positions). Hits `kettle` (sub-number:
  the kettle) and `interval`. Recorded per frame as `KETTLES`: each kettle `(owner, i, x, y, radius)` and each live
  ring `(owner, i, x, y, radius now, reach)`.

## Abilities

- **PEDAL SLIDE** (new kind `pedal_slide`, unlock 1, `every_bars` 4): for `dur_beats` beats every kettle glides
  toward the enemy at `speed`; the first touch of each kettle squeezes it (`dmg`, `knock` away from that kettle).
  The kettles stay where they stop. Event `pedal_slide`.
- **CROSS-STICKING** (`haste`, reused; unlock 2, `every_bars` 6): `mult` 4 for `dur_beats` 4: a hop every 8th.
- **FULL SET** (new kind `full_set`, below 25 HP): two more kettles appear at the other two corners of the diagonal
  square (`r_mid` each); from then the hops go round all four, and on every beat one kettle the timpani is not
  standing on booms by itself in turn. Event `full_set`.

New kinds live in the weapon's module (`bf_sim::kinds::kettles`), as MARIMBA's do. Existing kinds are not changed,
so no existing fight changes.

## Look and sound

- Look: the model as the ball's sprite; the kettles drawn on the floor as copper bowls seen from above (cream head,
  chrome hoop, rod ticks; the high one smaller); a hop as a short arc with a shadow; a boom as a copper ring from the
  kettle and a head flash; the interval hit as a bright chord flash between the two rings; PEDAL SLIDE as skid trails
  behind the kettles; FULL SET's new kettles dropping in.
- Sound: a timpani voice (a pitched membrane: a low fundamental with the strong 1.5 and 2 overtones, a felt thump
  and a long ring); the low kettle plays the chord's root and the high one its fifth; the interval hit both at once;
  PEDAL SLIDE a pedal glissando; CROSS-STICKING fast alternating strokes; FULL SET a four-drum rolling fanfare. Its
  personality motifs (invented): the low-high "boom-bah" call, a root-fifth-octave climb, a tuning-glissando fill.

## Numbers (first guess; the seed finder balances)

every 2, air16 2, spot 0.5, r_low 0.6, r_high 0.45, r_mid 0.52; speed_low 4.5, reach_low 2.6, dmg_low 2.6,
knock_low 1.2; speed_high 8.0, reach_high 1.6, dmg_high 1.9, knock_high 0.8; pair_t 0.6, dmg_pair 2.2. PEDAL SLIDE:
dur_beats 2, speed 2.4, dmg 2.0, knock 1.0. CROSS-STICKING: mult 4, dur_beats 4. FULL SET: r_mid 0.52.

## Plan

1. Engine: `bf-core` roster kinds (`kettles`, `pedal_slide`, `full_set`), `roster_new.json` entry; `bf-sim` kind
   module, sockets (setup, tick, later step, record), unit test; a quick-fights test against 4 fighters.
2. Content: `fighters/timpani/` (look, sound, voice, card, state, personality), model JSON, registry/NEW lists,
   cards section, fit traits, starter pairing and home theme, colour table in `docs/ballfight/RULES.md`.
3. Model: on Aedan's PC from the reference photos (generator script beside the other new models).
4. Proof (lenient): `cargo fmt`, clippy, `cargo test --workspace` (bf-app type-checked for Windows); GitHub's Windows
   CI. Rebase onto the latest build branch, merge, push. No install.
Quick fights (5 foes x 4 seeds x both corners): TIMPANI wins 24 of 40, every fight a K.O.
