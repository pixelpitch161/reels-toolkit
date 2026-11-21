# BASS GUITAR fighter: design (Rock Pick)

Approved by Aedan on 2026-11-21 (picked idea C "Rock Pick" from the animated concepts,
https://claude.ai/artifact/SFA3vajmPkAwf4MtricxSY, colour ultraviolet).

## Intent

A new from-scratch Studio fighter: the four-string electric bass, played with a pick by a rock bassist. It must stay
clearly apart from CONTRABASS (upright: WALKING BASS, SLAP BASS), ELECTRIC (LIGHTNING RIFF) and the OCTOBASS being
designed at the same time (slow floor ripples, deep umber). Engine 1 style only: flat effects over the ring, the ball
is a low-poly model built from real web photos of an electric bass. Aedan likes hits that stop dead on contact, so
three of its four moves do.

## Approaches considered

- A Thump & Pop (funk neck jabs and string pops): close to CONTRABASS's SLAP BASS and TROMBONE's thrust.
- B Sub Woofer (wobble beam, speaker cabinet, filter sweep, sub quake): overlaps OCTOBASS's floor ripples.
- **C Rock Pick (picked):** the most unlike any of the 40 fighters; a boomerang projectile plus three
  stop-on-contact hits.

## The fighter

- Name `BASS GUITAR`, colour ultraviolet (150, 0, 255), dark (82, 0, 140). Corner blue. Section Strings, plays bass.
- Weapon **PICK TOSS** (new kind `pick`): every `every` beats a big pick is thrown on a curved loop: it swings out to
  the enemy's spot (`lead` x its velocity ahead), passes it and curls back to the ball over `flight_beats` beats. A
  pick hits once (`dmg`, knock `knock` along its flight). It starts with `n0` pick per throw; each landed pick adds
  `grow` (up to `n_max` = 3); extra picks fly in a fan, bending to alternate sides.
- Level 2 **FRET SLIDE** (`fret_slide`): the ball holds still for `wind_beats` while fret marks light up along the
  line to the enemy, then dashes along it at `speed` x the shared speed, up to `span` ball radii. It stops dead on
  contact: `dmg`, knock `knock`, and the ball stays put for `settle_beats`, then glides on backwards.
- Level 3 **HEADSTOCK SWING** (`headstock`): the neck grows to `len` ball radii and swings back over `wind_beats`, then
  sweeps `sweep` degrees round to the enemy over `swing_beats`. It stops dead on the enemy: `dmg`, knock `knock`
  (out and along the swing), the biggest single hit of the kit; then the neck shrinks back.
- Low **SLINGSHOT** (`slingshot`, below 25 HP, once): two strings tie to the two corner posts behind the ball, it
  draws back `back` ball radii over `pull_beats` (strings humming), then fires itself at the enemy at `speed` x the
  shared speed (up to `span`), stopping dead in it: `dmg`, knock `knock`.
- The three moves keep their numbers when fired, so a copied one (EUPHONIUM's IDENTITY CRISIS) works for any ball.
- A dash ends early when the ball is stunned, held, grabbed, pinned, airborne or smashed, or reaches the ropes.

Starting numbers (tuned by the quick-fights test; balance is lenient, the Studio handicaps pairs):
pick `every 2, flight_beats 1.5, lead 0.5, bend 0.55, fan 0.45, n0 1, grow 0.25, n_max 3, dmg 2.4, knock 0.5,
hit_r 0.22`; FRET SLIDE `wind_beats 0.5, speed 3.0, span 6, dmg 4.5, knock 1.2, settle_beats 0.25, every_bars 4`;
HEADSTOCK SWING `len 2.2, sweep 150, wind_beats 0.75, swing_beats 0.25, dmg 6.5, knock 1.8, every_bars 5`;
SLINGSHOT `pull_beats 2, back 1.5, speed 4.0, span 9, dmg 9, knock 2.0`.

## Engine

`bf-core/src/roster/kinds/pick.rs` (settings), `bf-core/src/roster/abilities/{fret_slide,headstock,slingshot}.rs`,
`bf-sim/src/kinds/pick.rs` (the picks, the dashes, the swing; no generator draws, no socket except `snap.PICK`).
While dashing or drawing back the ball is `walking` (its own step moves it; nothing knocks it). Events: `pick`
(throw), `fret_slide`, `headstock`, `slingshot` (start, then 1 when it lands). Record key `PICK`, one row per pick in
the air, dash, swing or slingshot.

## Content

`bf-content/src/fighters/bass_guitar/`: look (model as the ball; ultraviolet picks with a motion trail; glowing fret
bars on the slide line; a long maple neck with headstock for the swing; two humming strings for the slingshot),
sound (a picked electric bass: root notes on each throw, a slide up the neck for FRET SLIDE, a low power-chord thump
for HEADSTOCK SWING, a rising string stretch and a hit for SLINGSHOT), voice (Karplus-Strong low pluck, dark, with a
pick click), card, personality (driving eighth-note rock bass), VS-card panel (`shared/panels.rs`: diagonal strings
over fret bars). Model from Wikimedia photos of electric basses, built on Aedan's PC.

## Non-goals

No install. No changes to other fighters. Not the retired GUITAR.
