# SNARE DRUM fighter: design

Aedan asked (2026-11-16) for a new fighter, SNARE DRUM, made from scratch (no Python kit reference: Rust is the source
of truth). Three move sets were drafted with visual concepts (https://claude.ai/artifact/7C5SqvDGn5onJKaBnNWo5z) and
Aedan picked **A, Stickwork** (2026-11-16 15:53Z). Process rules: once the design is approved, build it through to a
merge into the build branch without stopping; do not install; a lenient balance check (fights auto-balance by
handicap and seed); the model is made on Aedan's PC from real web photos of snare drums.

## Approaches considered

- **Stickwork** (picked): two drumsticks jab in a paradiddle sticking. The clearest picture of a snare; every move is a
  real snare technique; nobody else attacks in a sticking pattern.
- Drumline: thrown sticks that ricochet off the walls. Close to MARIMBA's ricochet mallets and other projectile
  fighters; its march overlaps SOUSAPHONE.
- Backbeat: forward cracks on 2 and 4. Waves read too much like DRUMS' pulse and BASS DRUM's HEARTBEAT.

## The fighter

- Name `SNARE DRUM`, colour **Cherry sparkle** rgb(255, 48, 128) (dark rgb(150, 18, 70)). The reds are taken (KAZOO
  red reserved, SOUSAPHONE crimson, ORGAN wine, BARI SAX coral, CORNET rose); cherry sits between wine and pink and is
  far more saturated than CORNET's rose. Rechecked against the sibling fighters' colours before the merge.
- Distinct from DRUMS (round drumhead pulse, DRUM ROLL haste, CRASH CYMBAL, DOUBLE KICK), BASS DRUM (rolling charge,
  HEARTBEAT, DRUM ROLL zigzag, 1812 cannons) and TIMPANI (tuned booms): the snare is sharp, dry, high and fast.
- Model: a marching/concert snare built from web photos on Aedan's PC (chrome hoops and tension lugs, a sparkle-finish
  shell, a white coated head, a pair of crossed sticks optional), `bf-content/data/models/SNARE DRUM.json`, generator
  `studio/scripts/models/snare_drum_model.py`, notes and photos in `studio/reference/models/snare_drum/`.
- Unpitched; idiom `rudiment` (its part plays rudiments over the beat, never the melody). Register field: a high
  unpitched range like DRUMS'.

## Weapon: PARADIDDLE (new kind `sticks`)

Two sticks ride the ball, one on each side of the line to the enemy (left stick at -`spread` degrees, right at
+`spread`). Every `step16` sixteenths one stroke is played, following the paradiddle sticking R L R R, L R L L (eight
strokes, then repeat; haste shortens the step like any grid). A stroke is a jab: over `out16` sixteenths the stick
shoots out from the ball's edge along its side's direction to `reach` (in ball radii: `base` + `grow` per landed hit,
up to `max_reach`), then pulls back. If the stick's segment touches the enemy while going out, it hits once:
`dmg` (x `accent` on the first stroke of each group of four), a knock of `knock` along the jab. Events `stroke`
(side, accent) and hits of kind `stick` (sub-number: 1 for an accent, 0 otherwise). Recorded per frame as `STICKS`:
each stick's side, extension 0..1 and its tip, so the look and the part read the same thing.

## Abilities (new kinds, in the weapon's module)

- **FLAM** (`flam`, unlock 1, every `every_bars` bars): for `dur_beats` beats every landed jab is a flam: a second
  hit of kind `flam` worth `mult` of it. The look draws a faint grace stick just ahead of each jab.
- **RIMSHOT** (`rimshot`, unlock 2, every `every_bars` bars): arms the snare for up to `dur_beats` beats; the next
  jab that lands is a rimshot: damage x `dmg_k`, knock x `knock_k`, and the enemy is stunned for `stun_beats` beats
  (the engine's stun: holds still, starts no attack). Event `rimshot`; a bright ring on the enemy.
- **BUZZ ROLL** (`buzz_roll`, below 25 HP): for `beats` beats the sticks blur into a buzzing disc of radius `radius`
  around the ball (the weapon pauses); every `tick16` sixteenths the enemy, if it touches the disc, takes `dmg` and a
  small knock `knock`. Hits of kind `buzz`.

Starting numbers (the builder tunes them in a lenient check): `step16` 2, `out16` 1, `spread` 35, `base` 1.2,
`grow` 0.06, `max_reach` 2.4, `dmg` 1.4, `accent` 1.5, `knock` 0.35; FLAM `dur_beats` 8, `mult` 0.5, `every_bars`
4; RIMSHOT `dur_beats` 8, `dmg_k` 3.0, `knock_k` 2.5, `stun_beats` 1, `every_bars` 5; BUZZ ROLL `beats` 4, `radius`
2.2, `tick16` 1, `dmg` 0.8, `knock` 0.12.

## Look and sound

- Look: the model as the ball's sprite; the two sticks drawn over the ring (pale hickory shafts with a nylon/wood
  tip, a light trail on each jab, a spark on an accent hit), a faint grace stick under FLAM, a bright crack ring and a
  short flash on the enemy for RIMSHOT, a spinning blurred disc in its colour for BUZZ ROLL, wood-splinter and head
  shards when it is knocked out.
- Sound: a snare voice (a short tuned head thump around 200 Hz plus a bright noise burst through a band filter for the
  wires, a brighter, louder variant for rim shots, a tight press-roll buzz); each stroke plays a tap (accents louder),
  a flam a grace tap a hair before, RIMSHOT a dry crack, BUZZ ROLL a press roll. Personality motifs are invented for
  it: a marching figure (a flammed accent on beat 1, sixteenth diddles, an accented rimshot on 4) and a solo that
  builds from paradiddles into a buzz roll and ends on a single rimshot.

## Calls left to the builder

Exact numbers after a short lenient balance run against a few opponents. Card text, tags, theme fit traits and the
title-card pattern follow the other fighters' modules (MARIMBA is the nearest template).
