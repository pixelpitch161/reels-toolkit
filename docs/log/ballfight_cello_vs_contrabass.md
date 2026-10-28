# Log: `projects/ballfight_cello_vs_contrabass`

Append new entries at the end (newest last).

## #47 BALLFIGHT: Cello vs Contrabass (session_01CzTfqb, Oct 28), an episode-kit episode with two new fighters
- Made with the episode kit: one `episode.py` (seed, scale, mood, D minor tango loop Dm A7 A7 Dm Gm Dm E7 A7, the
  CONTRABASS's walking line, the CELLO's melody, both solos, title hits, post text). The low strings: the swan vs the
  elephant (Saint-Saens' Carnival of the Animals). No episode number on screen.
- **CONTRABASS model from reference photos (Claude in Chrome):** Commons' "AGK bass1 full.jpg" (CC BY-SA 3.0) has one
  bass straight-on from the front AND the side on white; drawn into a canvas from a Commons page (same-origin API for
  the file list, `crossOrigin` image) and measured per pixel row (body 887 px): sloping shoulders, corners at 65 / 48 %,
  lower bout .62 L wide, f-hole eyes, bridge at 48 %, tailpiece, fingerboard end; side view: ~.22 L deep, the flat
  back breaking forward over its top fifth, the tall bridge. The Met's CC0 double bass (front, back, scroll, three-
  quarter) as the cross-check; head + neck set to .60 L (AGK .56, Met .71). `series/instrument-history/contrabass_lowpoly.py`,
  model, voice and bridge entry in three commits. Size: the bridge's factor x1.2 (and `sprite_k` 0.84) plus the cello at
  `sprite_k` 0.74 make the bass ~1.5x the cello in their glasses; above r ~ 20 px the x1.2 passes `render_sprite`'s 3 px
  margin, so the Look renders a padded crop (as #42's viola).
- Voice (`contrabass_lowpoly`): additive partials through low body resonances (60 / 110 / 220 Hz, a nasal band at 500,
  rosin band at 1.1 kHz), a weak fundamental on the low strings; arco (rosiny chuff, slow vibrato), pizz (thump + boom,
  high partials die first), slap (a click and a fingerboard clack over a bright pizz; unpitched ghost slap). The bridge's
  voice() is pizz < 0.5 s, else arco; the kit Sound calls the three directly.
- **Engine (each its own commit, replay guard identical every time):** `walk` (one heavy step a beat, footfall ON the
  grid; WALKING = knock() ignores it, a grapple slips, it never gives way in a clash: the other ball bounces off, one
  clash event per push), `grapple` + `vibrato` (rush 2 16ths at 2.6 V right after the bar line, grab within reach,
  hold a beat swinging the enemy round, throw at the nearest rope, rope slam within 0.9 s), `slap` (waits for the enemy
  or fires at once when held: breaks the grab) + `prelude` (a note-wave ring every 16th for a bar, reaches rocking with
  Bach's figure), `swan` (rise, glide on a Bezier curve, dive; airborne = no hits / knocks / clashes) + `stampede`
  (unstoppable charge, trample + charge on past, quake every 8th). Bugs found in testing and fixed before the roster:
  a planted walker taking half the overlap trapped a ball against the ropes (120 rope events / s), so a walker never
  gives way; a grab at the ropes swung the enemy through them and the release could land on the grappler (the walker
  then shoved it a metre), so the hold shuffles in to 3 R off the ropes; a stampede pinning a ball in a corner squeezed
  it out, so a walker stops short against a pinned ball and tramples charge on past.
- Kit (additive, own commits): `st['lift']` (a look lifts its ball: ball, painter depth and canvas shadow at R + lift;
  0 elsewhere, example frames byte-identical), HUD holds for the new kinds, GRAB / WALK as arrays. Tried making the speed
  lines ignore the lift (the take-off's lines looked like stilts): with no motion the ability moment's lines fall back
  to streaming above the ball (reads as falling), so it was reverted; the real lift motion is the right look.
- Look: CELLO's bow arm reaches out on the rush and wraps round the far side of the enemy on the grab (rust swing
  trail, rosin dust, rope-slam bursts), VIBRATO zig-zags and a trembling held enemy lifted off its feet, PRELUDE's amber
  rings with riding eighth notes, SWAN SONG's white wings and falling shadow, a splash ring + foam + feathers on the dive.
  CONTRABASS: lifting steps, dust, quake rings, footprints, a snapping string on SLAP BASS, elephant ears (grey: steel-
  blue ears vanished on its glass) and a curling trunk, dust plumes and cracks on THE ELEPHANT. VS card: f-holes + bow
  hair vs footprints.
- Previews (55 s each) sent before tuning.
- **Mood `dark_tango`** (new, `kit/moods/`): bandoneon marcato (two beating reeds, nasal formants; sincopa + arrastres
  in the back half), a muted nylon guitar for the strings' chop, golpe + chicharra + latigo, no piano, no bed bass. Tango
  stops: the band hits and cuts for the beat on every stop a fighter asks for (Sound.stops(): the cello's rope slams).
  The walking line is the bass's footfalls (DOUBLE TIME's 8ths add chromatic approaches), the cello sings the melody
  (repeats tied into long bows). Quotes: Bach's Prelude bar 1 as chord degrees per note-wave (exact), The Swan's opening
  phrase (pitches from a letter-notes transcription, rhythm ours) and The Elephant's waltz tune (from memory; flagged to
  the user) in the key's harmonic minor.
- Tuning (kit tune, seeds 1000..): the first numbers gave CELLO ~80 % with ~0.3 lead changes (the bass's quakes rarely
  reached a ball gliding faster than it walks); a wider quake flipped it to the bass ~100 % because the quake's knock
  interrupted every rush. Frozen: quake reach 0.66, knock 0.2, scale 0.85: CELLO 46 %, median K.O. 49.3 s, both
  desperation moves in 78 % of fights, fit ~1 %. Search 1..3504 (two ranges): 22 fits; none captions both desperation
  moves (they fire ~2 s apart, 4 s rule). **Seed 2455**: CELLO leads early on grabs and VIBRATO / PRELUDE, the bass takes
  over on DOUBLE TIME, SWAN SONG (dive lands at 40.9 s) swings it back, THE ELEPHANT answers at 41.2 s, CONTRABASS wins
  at 50.88 s with 0.1 HP (lead changes 32.6 / 40.9 / 42.8 s). Unlock times repeat across seeds (grabs on the bar line
  quantise the damage), the fights diverge from ~30 s.
- Sound lessons: the band alone sat ~3.5 dB under the fight after the K.O. (levels FAIL, two dips at 56 / 58 s): the
  victory now takes up the melody fighter's line on the bandoneon, a chop on the 8ths, a denser last bar, fuller
  homecomings: PASS (opening -0.1 dB vs the ending, spread 4.8 dB, no dips). The intro read -1.8 LU under the fight:
  the fighters carry the card and count-in (the cello's swell holds through VS, a long bow under the count, the bass
  walks the card's last beat), the full marcato on bar 1: -1.1 LU.
- Checks (`python -m ballfight check`): safe zone PASS, focus CoC 0 px on 1,830 frames, levels PASS, band CELLO
  +9.5..+12.5 / CONTRABASS +9.9..+15.4 dB over the bed, flash 0 (max 12.0 % of the frame), loop PASS (seam 0.4x, audio
  1.0 dB; the usual reassembly warn at 56.9 s).
