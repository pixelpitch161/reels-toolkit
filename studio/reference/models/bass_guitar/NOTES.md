# BASS GUITAR model: photo references

Photos downloaded from Wikimedia Commons (960 px thumbnails, or the original where it was smaller; the 1966 Jazz Bass
as the 500 px thumbnail, as Commons refused the original at the time) into `photos/`. Each file page names its author
and licence.

| File | Source | What it shows |
| --- | --- | --- |
| `photos/Fender_AP_P-Bass_20230801.jpg` | https://commons.wikimedia.org/wiki/File:Fender_AP_P-Bass_20230801.jpg | A Fender American Performer Precision Bass standing on the grass, straight from the front: white body, black pickguard, rosewood board, maple headstock. The main reference: the body outline, the pickguard and the neck/body proportions were traced from it (26 px to the inch). |
| `photos/Fender_Precision_Bass.jpg` | https://commons.wikimedia.org/wiki/File:Fender_Precision_Bass.jpg | A vintage sunburst Precision Bass, front and back: tortoise guard, split pickup, chrome bridge cover, two knobs; the back shows the maple neck and the neck plate. |
| `photos/Fender_Jazz-Bass_1966.jpg` | https://commons.wikimedia.org/wiki/File:Fender_Jazz-Bass_1966.jpg | A 1966 Jazz Bass in Olympic White, full length from the front: the offset waist, tortoise guard, two single-coil pickups, the chrome control plate with three knobs, the cloverleaf tuner keys. A second check on the length (34-inch scale) and the headstock. |
| `photos/Fender_AP_P-Bass_Headstock_20230730.jpg` | https://commons.wikimedia.org/wiki/File:Fender_AP_P-Bass_Headstock_20230730.jpg | Close-up of the P-bass headstock: the 4-in-line outline, four chrome posts with bushings, the cloverleaf keys standing out on the bass side, the string tree, the bone nut. The headstock outline and the post and key positions were traced from it (91 px to the inch). |
| `photos/1972_Fender_Precision_Bass_Body.jpg` | https://commons.wikimedia.org/wiki/File:1972_Fender_Precision_Bass_Body.jpg | Close-up of a 1972 Precision Bass body: the tortoise guard round the neck end, the chrome pickup and bridge covers, the two knobs and the jack, the bevelled black edge of the sunburst. |

## What the photos show

- **Proportions.** A 34-inch scale (nut to saddles); the whole bass is about 44 to 46 inches long, the body about 20
  inches long and 13 across the lower bout, and the neck stands out of the body about as far as the body is long
  (about 21 inches from the upper horn to the nut). The headstock is about 8 to 9 inches long. The neck is 1.75 inches
  across at the nut and about 2.4 at the heel; the body is 1.75 inches thick.
- **Body.** An offset double cutaway: the bass-side horn (left, seen from the front with the headstock up) is long
  and reaches past the neck pocket; the treble-side horn is shorter and the cutaway under it deeper; the waist is
  offset, the lower bout round and wide. The edges are rounded over (on the sunburst, the darker colour rolls round
  the edge).
- **Pickguard.** It wraps round the neck end, runs down the bass side to below the pickup and down the treble side
  past the knobs to near the bottom, leaving the bridge on bare body.
- **Hardware.** A split pickup in two staggered halves (the bass-side half nearer the neck), a bridge with four
  saddles, two chrome knobs (volume, tone) and the jack on the guard's lower horn.
- **Neck and headstock.** Four strings over 20 frets; dot inlays at frets 3, 5, 7, 9, 12 (double), 15, 17, 19. The
  4-in-line headstock has its four posts in a slanting line and the big cloverleaf keys standing out on the bass
  side; the string nearest the nut is the low E, the G goes to the top post; a string tree holds the D and G.
- **Colours.** Body finishes vary (the photos: white, Olympic White, sunburst); the fighter's is ultraviolet
  rgb(150, 0, 255) with darker facets rgb(82, 0, 140) on the bevel and the sides. The guard is white (the AP's is
  black, the vintage ones tortoise; white reads best against ultraviolet). The neck is maple (the AP and the Jazz have
  rosewood boards; a maple board reads lighter at ball size), with dark fret lines and black dots, as on maple-board
  basses.

## Model and previews

`studio/scripts/models/bass_guitar_model.py` writes `studio/crates/bf-content/data/models/BASS GUITAR.json` (760
faces). It builds the bass flat in inches from the traced outlines, face to the camera, then turns it -40 degrees
about z (the headstock up and to the right, the body low on the left), tips its face 18 degrees up towards the
camera and turns it -30 degrees about y, which cancels a roster `view` yaw of 30 so the bass lies face on to the
camera (as PICCOLO does). Suggested roster `view` yaw: 30. Across the instrument the neck is drawn 1.5 times wider
than life and the headstock 1.3 times, the body a tenth bigger and 2.4 inches thick, so they read at a small radius;
along it, the 34-inch scale is kept, so the neck is a little longer than the body. The body has a bevelled edge of
facets (every third one darker), dark sides and a dark back, so it reads as a solid from every yaw. The strings are
thin two-sided faces (lines are not part of a model).

`render_r100_yaw30.png` is radius 100 px at x2, `render_r40_yaw30_x3.png` radius 40 px at x3, both drawn with the
series rasteriser (`series/instrument-history/piano_lowpoly.py`: `render_geo` + `outline`) through the sprite camera
of `projects/which_instrument_wins/sprites.py` (elevation 16 degrees), exactly as SNARE DRUM's previews were (that
set-up reproduces SNARE DRUM's committed previews pixel for pixel).

Refinements after comparing with the photos: the fret lines were made thinner and lighter and the dots a little
smaller (the first pass read as a dark ladder and speckled the neck at radius 40); the cloverleaf keys were made
bigger and moved out from the headstock's edge, as they stand in the headstock photo; then the body outline and
pickguard were made a tenth bigger, so the ultraviolet body reads at radius 40 next to the long neck.
