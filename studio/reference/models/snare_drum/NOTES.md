# SNARE DRUM model: photo references

Photos downloaded from Wikimedia Commons (960 px thumbnails, or the original where it was smaller) into `photos/`.
Each file page names its author and licence.

| File | Source | What it shows |
| --- | --- | --- |
| `photos/Snare_drum_-_Vladimir_Morozov.jpg` | https://commons.wikimedia.org/wiki/File:Snare_drum_-_Vladimir_Morozov.jpg | Kit snare (Sonor Force 3007, blue burst) from slightly above, a pair of hickory sticks lying on the white coated head. The main reference for the view and the sticks. |
| `photos/2006-07-06_snare_14.jpg` | https://commons.wikimedia.org/wiki/File:2006-07-06_snare_14.jpg | 14-inch kit snare straight from the side on a stand: chrome hoops, tube lugs with tension rods running to both hoops, the strainer on the left, a chrome air-vent badge. The main reference for proportions. |
| `photos/Snare_drum_strainer.jpg` | https://commons.wikimedia.org/wiki/File:Snare_drum_strainer.jpg | Close-up of a snare strainer (throw-off) with its lever and tension knob, the triple-flange hoop and a coated head. |
| `photos/Drumhead_Coated_on_Snare_Drum.png` | https://commons.wikimedia.org/wiki/File:Drumhead_Coated_on_Snare_Drum.png | A snare from high above: the white coated batter head filling the hoop, lugs around a brass shell, the strainer at the side. |
| `photos/Marching_snares.jpg` | https://commons.wikimedia.org/wiki/File:Marching_snares.jpg | A marching band's snare line: deep white shells, bright heads, rods running the full depth. Character reference only. |

## What the photos show

- **Proportions.** A kit/concert snare is 14 inches across and 5 to 6.5 inches deep (diameter : depth about 2.2 to
  2.8 : 1); marching snares are deeper, about 14 x 12. The model uses depth 0.88 of the radius (about 14 x 6.2),
  a little deep so the cherry shell still shows at ball size.
- **Lugs.** Ten lugs (5 per side when seen from the side), evenly spaced, sitting at the middle of the shell; on tube
  lug drums a tension rod runs from each lug up through the top hoop's claw and another down to the bottom hoop, the
  rod heads standing just above (and below) the hoops.
- **Hoops.** Triple-flange pressed chrome: a band a little wider than the shell, standing above the head with a
  rolled-out rim and a step in the band. Both hoops are the same; the bottom one sits flush with the shell's edge.
- **Head.** White coated, slightly off-white and matt, sitting just below the hoop's rim.
- **Strainer.** A black (or chrome) block between two lugs with a chrome throw-off lever standing up and a tension
  knob on top; a small butt plate sits on the opposite side.
- **Colours.** Shell finishes vary (the photos: blue burst, teal fade, brass, white); the fighter's is cherry
  sparkle rgb(255, 48, 128) with darker sparkle facets rgb(150, 18, 70). Chrome is light silver; the sticks are pale
  hickory with a slightly darker wood tip.
- **Sticks.** About 16 inches, a little longer than the head's diameter, so they overhang the rim when laid on the
  head. The model crosses them in an X so both read at small size.

## Model and previews

`studio/scripts/models/snare_drum_model.py` builds the drum upright and tips it 20 degrees towards the camera so the
head reads as a wide oval; keep the look's yaw near the front (the previews use yaw 30). `render_r100_yaw30.png` is
radius 100 px at x2, `render_r40_yaw30_x3.png` radius 40 px at x3, both drawn with the series rasteriser
(`series/instrument-history/piano_lowpoly.py`: `render_geo` + `outline`) through the sprite camera of
`projects/which_instrument_wins/sprites.py` (elevation 16 degrees), exactly as MARIMBA's previews were (that set-up
reproduces MARIMBA's committed previews pixel for pixel).

One refinement after comparing with the photos: the hoops were made taller (the first pass read as thin wires and
the bottom hoop looked detached from the shell), the tilt went from 26 to 20 degrees so more of the cherry shell
shows, and the sticks got a little thicker.
