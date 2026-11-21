# OCTOBASS fighter: design

Aedan asked (2026-11-21) for a new fighter, OCTOBASS, made from scratch (no Python kit reference: Rust is the source
of truth). Three move sets were drafted as animated loops against the target dummy
(https://claude.ai/artifact/W7TG6U88kk7KNVh73XAuU2) and Aedan picked **A, Lever Machine, with C's low move BELOW
HEARING** (2026-11-21 18:12Z). Process rules: once the design is approved, build it through to a merge into the build
branch without stopping; do not install; a lenient balance check (fights auto-balance by handicap and seed); the
model is made on Aedan's PC from real web photos of octobasses; Engine 1 look only.

## Approaches considered

- **Lever Machine** (picked): the octobass's real lever-and-pedal mechanism (fingers can't reach its strings) turned
  into lever arms that punch and stop dead on contact (Aedan liked contact-stopping punches on the Trumpet rounds).
  Nothing in the roster punches with arms.
- The Giant: a giant bow, a stepladder drop, growing, a toppling TIMBER!. Size-based; the bow overlaps the string
  fighters and the drop overlaps slam abilities.
- Felt, Not Heard: air bubbles, buzzing ropes, jelly floor, a silent ring then BOOM. Its low move was kept.

## The fighter

- Name `OCTOBASS`, colour **deep umber** rgb(100, 64, 36), dark rgb(52, 32, 16): old dark wood. Its nearest colours
  by CIELAB distance are CONTRABASSOON and HORN (about 35); the BASS GUITAR thread (in progress) has ultraviolet
  (150, 0, 255) with fallback olive (120, 120, 40), also about 35 away.
- Distinct from CONTRABASS (WALKING BASS, SLAP BASS, THE ELEPHANT stampede) and CELLO (grapple): the octobass is a
  machine, its arms are levers, its low move is silence.
- Model: an octobass built from web photos on Aedan's PC (a 3.5 m violin-family body, very tall neck and scroll, three
  strings, the row of metal levers up the neck, foot pedals at the base, an endpin), `bf-content/data/models/OCTOBASS.json`,
  generator `studio/scripts/models/octobass_model.py`, notes and photos in `studio/reference/models/octobass/`.
- Pitched, the lowest register in the game; idiom `sub` (its part plays long, very low roots, an octave under the
  bass line).

## Weapon: LEVER PUNCH (new kind `levers`)

Every `every16` sixteenths the octobass fires a volley of `n` lever punches, one every `gap16` sixteenths. `n` starts
at `n0` and grows by `grow` per landed punch up to `n_max` (3, one per string). Each punch is aimed at the enemy when
it fires, with a side offset of -`spread`, 0, +`spread` degrees for arms 1, 2, 3. Over `out16` sixteenths the felt
fist shoots from the ball's edge straight along its line up to `reach` metres; **if it touches the enemy it stops
there** (the tip holds at the contact point for `hold16` sixteenths, the enemy is not passed through), hits once for
`dmg` with a knock of `knock` along the line, then the arm pulls back over `back16`. Events `lever` (arm) and hits of
kind `lever`. Recorded per frame as `LEVERS`: each live arm's angle, extension (0..1), tip and whether it is held on
contact, so the look and the part read the same thing.

## Abilities

- **FOOT PEDAL** (`foot_pedal`, unlock 1, every `every_bars` bars): the octobass stomps; a pedal plate appears where
  the enemy is. After `tele` beats it snaps up: if the enemy is within `radius` of the plate it takes `dmg` and is
  stunned for `stun_beats` beats (the engine's stun). Events `pedal_set`, `pedal_snap`; hit kind `pedal`. The look
  pops the enemy up in the air (a lift on the sprite and a shadow) while stunned, then a landing squash.
- **SEVEN LEVERS** (`seven_levers`, unlock 2, every `every_bars` bars): all seven levers fire one after another,
  `gap` seconds apart, fanned over `fan` degrees around the line to the enemy and converging on it. Each that reaches
  the enemy stops on contact for `dmg`; the seventh adds a knock of `knock_last`. The normal weapon pauses while it
  runs. Hits of kind `seven`.
- **BELOW HEARING** (`below_hearing`, below 25 HP): for a moment everything goes quiet: the music drops out and the
  ring darkens while one slow ring grows from the octobass at `speed` m/s. When it reaches the enemy (or after
  `max_s` seconds) the silence breaks: `dmg`, a knock `knock`, a stun of `stun_beats`, a light flash and a shake. The
  music comes back on the hit. Events `below_start`, `below_hit`; hit kind `below`. The part/mix reads `below_start`
  to `below_hit` as a silent stretch (only a very low hum and the octobass's own sub note).

Starting numbers (the builder tunes them in a lenient check): `every16` 8, `gap16` 2, `n0` 1, `grow` 0.25, `n_max` 3,
`spread` 12, `out16` 1, `reach` 2.6, `hold16` 1, `back16` 2, `dmg` 1.4, `knock` 0.5; FOOT PEDAL `tele` 1, `radius`
0.9, `dmg` 2.5, `stun_beats` 1, `every_bars` 5; SEVEN LEVERS `gap` 0.09, `fan` 60, `reach` 3.0, `dmg` 0.9,
`knock_last` 1.6, `every_bars` 6; BELOW HEARING `speed` 2.0, `max_s` 3.0, `dmg` 13, `knock` 2.0, `stun_beats` 1.

## Look and sound

- Look: the model as the ball's sprite; lever arms drawn over the ring (a dark wood rod with a brass edge, a knuckle
  joint that folds when the arm is short, a dark red felt fist), a thud burst where a fist stops on the enemy; a brass
  pedal plate under the enemy for FOOT PEDAL; a fan of seven arms for SEVEN LEVERS; for BELOW HEARING a dark veil over
  the ring and a wide slow wood-coloured ring, then a flash and a big burst; wood splinters and a snapped string on a
  knock-out.
- Sound: a deep bowed and plucked sub-bass voice (a fundamental near 30 to 60 Hz with strong low harmonics so it
  reads on phone speakers), a wooden clunk plus felt thud for each lever punch, a metal pedal snap, seven fast
  clunks, and for BELOW HEARING the music cut to near silence and a felt-not-heard rumble, then one huge BOOM.
  Personality motifs are invented: a slow giant's walk in the lowest register, and a solo that climbs the three
  strings one lever at a time.

## Calls left to the builder

Exact numbers after a short lenient balance run against a few opponents and the target dummy. Card text, tags,
theme fit traits and the VS-card panel pattern (`fighters/shared/panels.rs`: tall wood grain with a row of brass
lever bars) follow the other made-from-scratch fighters' modules (SNARE DRUM is the nearest template).
