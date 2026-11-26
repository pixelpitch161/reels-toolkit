# PICCOLO model: photo references

Photos downloaded from Wikimedia Commons (960 px thumbnails) into `photos/`. Each file page names its author and
licence.

| File | Source | What it shows |
| --- | --- | --- |
| `photos/Yamaha_Piccolo_YPC-81.png` | https://commons.wikimedia.org/wiki/File:Yamaha_Piccolo_YPC-81.png | Yamaha YPC-81: grenadilla body and headjoint, silver-plated keys, straight side view. The main reference for proportions, rings and key layout. |
| `photos/Piccolo_on_red_backdrop.jpg` | https://commons.wikimedia.org/wiki/File:Piccolo_on_red_backdrop.jpg | Grenadilla piccolo with a wave headjoint, lying diagonally, seen from above the keys: the cups, the rod and posts, the tall curved lever and the silver tenon ring. The reference for the diagonal pose. |
| `photos/Modern_flutes.jpg` | https://commons.wikimedia.org/wiki/File:Modern_flutes.jpg | A concert flute above a wooden piccolo at the same scale: the piccolo is about half the flute's length and has no foot joint. |
| `photos/Yamaha_Piccolo_YPC-82.png` | https://commons.wikimedia.org/wiki/File:Yamaha_Piccolo_YPC-82.png | Yamaha YPC-82: grenadilla body with a solid silver headjoint and lip plate. |
| `photos/Piccolo.jpg` | https://commons.wikimedia.org/wiki/File:Piccolo.jpg | Yamaha YPC-32 student piccolo: black resin body, silver-plated headjoint. |
| `photos/Mendini_C_Piccolo.jpg` | https://commons.wikimedia.org/wiki/File:Mendini_C_Piccolo.jpg | An all-metal (silver) student piccolo beside its case: raised lip plate round the embouchure hole. Character reference. |

## What the photos show

- **Size and joints.** A concert piccolo is about 32 cm long, about half a concert flute (Modern_flutes photo), and
  only 1.5 to 1.8 cm across. It has just two joints: the headjoint (crown to tenon, a little over a third of the
  length) and the body; there is no foot joint. The model is drawn at about 1 : 12 instead of 1 : 20 so it stays
  readable at a ball's radius.
- **Shape.** The headjoint is a near-cylinder with a gentle bulge round the embouchure hole (a "wave" head in the red
  backdrop photo). The body is conical on the outside: thickest at the tenon socket, narrowing towards the far end.
- **Colours.** Grenadilla is near-black with a hint of purple-brown (about RGB 34, 30, 34) and a soft polished sheen;
  the keys, rings, crown and lip plate are bright silver (about RGB 205, 210, 220).
- **Rings and crown.** A silver crown caps the head end with a silver ring under it; a wide silver ring sits at the
  tenon between head and body; a narrow silver ring finishes the open end of the body.
- **Lip plate.** On wooden headjoints the embouchure hole is cut into the wood with a slight raised wave; silver
  headjoints (YPC-82, Mendini) have a raised oval lip plate. The model gives it a silver oval lip plate, a little
  oversized, so the head end reads at a small radius.
- **Keys.** Closed cups in two hands of three along the body, with a smaller trill key near the end; each cup hangs on
  a short arm from one long rod held by posts on the near side; the tall curved lever (the long silver hook in the
  photos) stands up between the two hands. The model makes the cups and the lever a little bigger than life.

## The model

`studio/scripts/models/piccolo_model.py` writes `studio/crates/bf-content/data/models/PICCOLO.json` (304 faces). It
is built along its own axis, then turned -36 degrees about z (head end up and to the left) and -30 degrees about y,
which cancels a roster `view` yaw of 30 so the piccolo lies broadside to the camera, its cups and lip plate facing the
viewer and tipped 35 degrees up. Suggested roster `view`: `[30, 20, 0]`.

`render_r100_yaw30.png` (radius 100, shown 2x) and `render_r40_yaw30_x3.png` (radius 40, shown 3x) are the sprite
renderer's picture at yaw 30, roll 0, on black, made with a Python port of `bf-picture`'s `sprite::render_sprite`
that reproduces MARIMBA's previews pixel for pixel.
