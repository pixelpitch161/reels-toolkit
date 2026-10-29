# Log: `projects/ballfight_altosax_vs_tenorsax`

Append new entries at the end (newest last).

## #51 BALLFIGHT: Alto Sax vs Tenor Sax (session_01CzTfqcfepqv4wWSk4ryE2g, Oct 29), episode kit, two new fighters
- **Alto model check (Claude in Chrome):** Commons' "4 Saxophone Amati white background.jpg" (alto + tenor at one scale)
  and "Saksofon altowy Serie III GP firmy Selmer.jpg". altosax60 was off: bell too small, too high, too far out; neck too
  tall / long; rods, guards and levers drawn as lines (dropped by the sprites) and gold-on-gold cups, so at ball size a
  bare wire U. Rebuilt as `series/instrument-history/altosax_lowpoly.py` on the bari v2's construction (all faces, pad
  rings, pearls, rods on posts, guards with diamond feet, bands, engraving; `ball` scene tubes x1.45), with `sax(D)`
  taking a dimension table. altosax60 untouched (the retired SAX / #26 / #29 still use it).
- **Tenor:** `tenorsax_lowpoly.py` from Commons' "Yamaha Saxophone YTS-62 (recrop).png" + the Amati shot: x1.22, the
  neck's hump (mouthpiece pointing down), deeper bow, bigger bell set further out, bigger keys, amber lacquer; bridge size
  x1.06 vs the alto's x0.94. Voice `tenor_tone` (own commit): frequency-rolled harmonics through a 620 Hz husk / 1.5 kHz
  bark, a breath band, `honk` (scoop, growl, reed slap) and `sub` (subtone).
- **Engine (own commits, replay guard identical):** `spotlight` (a light sweeps square to the enemy's path every bar,
  burns on the 16ths, grows per landed sweep) + `octave` / `altissimo` / `encore` (chase light + phone lights);
  `crowd` (pose + honk, throws from the stands beyond the nearest rope, limited reach so the middle is safe, hype grows)
  + `runway` (strut, no knockback, honk rings) / `subtone` (hush, then x2) / `stagedive` (leap into the stands, surf,
  cannonball; airborne). Recorded SPOT / CROWD / THROWS.
- **Kit:** `kit/fighters/alto_sax.py` (pools + cones from the rafters, the arena dimmed only in a ring round each pool,
  scorch marks, phones; the bright lead: the line on each sweep, squeals on burns, ENCORE's 16th run),
  `kit/fighters/tenor_sax.py` (throws arcing out of the front rows, landing shadows, splats / petals / foam fingers, hype
  pips, catwalk with chaser bulbs, the stage dive's path through the stands; the honking riff, one note per throw).
  Mood `kit/moods/dark_disco.py`.
- Previews sent before tuning (pools first rendered blown-out white, toned down twice; catwalk dimmed). Tuned once:
  alto burn 1.3 (+0.06 to 2.1), tenor n0 3 / +0.34 / max 5, throw 1.9 (max 2.4), scale 0.75: 53 % / 47 %, median 50.2 s.
- Search 1..1011: 8 fits; SEED 621 (2nd) for the ENCORE vs STAGE DIVE finish. Fix after the first sheet: the catwalk
  outlived a strut cut short by STAGE DIVE. Tenor bus up / pad down for the band check. All checks + verify PASS.
