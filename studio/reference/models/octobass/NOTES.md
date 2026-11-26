# OCTOBASS model: photo references

Photos from Wikimedia Commons (960 px thumbnails, or the original where it was smaller), meant for `photos/`.
Each file page names its author and licence. The model was made by looking at these five photos at full size, but the
files themselves are not stored yet: on 2026-11-21 upload.wikimedia.org refused every download from Aedan's PC with
"429 Too many requests" for over half an hour. The file names below are the ones to save them under.

| File | Source | What it shows |
| --- | --- | --- |
| `photos/Octobass-Paris-2016.png` | https://commons.wikimedia.org/wiki/File:Octobass-Paris-2016.png | The Vuillaume octobass in the Musée de la Musique, Paris, from the front-left: red-brown varnish, sloping bass shoulders, f-holes and bridge, dark tailpiece, the long ebony fingerboard with the comb of stop levers on its upper end, the long rod beside the neck down to the hand levers sticking out at the left shoulder, the player's step at the base. The main reference for the view and the mechanism. |
| `photos/MIM_PHX_2011-04-26_0132_edited.jpg` | https://commons.wikimedia.org/wiki/File:MIM_PHX_2011-04-26_0132_edited.jpg | The Musical Instrument Museum (Phoenix) octobass straight from the front, full height: body outline, waist, f-holes, bridge, tailpiece, the stop levers up the fingerboard, the pegbox with big tuning keys and the scroll, the brass hand lever at the left shoulder. The main reference for proportions. |
| `photos/Octobass_MIM_PHX_4554573624.jpg` | https://commons.wikimedia.org/wiki/File:Octobass,_MIM_PHX_(4554573624).jpg | The same Phoenix octobass from further back, on its black plinth with the player's step and its lever box beside it. Reference for the base. |
| `photos/Octobasse_OSM_Eric_Chappell_1.jpg` | https://commons.wikimedia.org/wiki/File:Octobasse_Orchestre_Symphonique_de_Montr%C3%A9al_Eric_Chappell_1.jpg | The Montreal Symphony Orchestra's octobass being bowed by a player standing on a step: the scale against a person, the deep orange-brown varnish with its gloss, the lever row up the neck, the foot-pedal box at the base. |
| `photos/Octobass_Musee_de_la_Musique_Paris.jpg` | https://commons.wikimedia.org/wiki/File:Octobass_with_Lindsay_and_Domo,_Mus%C3%A9e_de_la_Musique_de_Paris.jpg | The Paris octobass with two visitors beside it. Character and scale reference. |

## What the photos show

- **Size and proportions.** About 3.5 m tall (the Phoenix label says 12 feet). The body is about 63 % of the height
  from the bottom block to where the neck leaves the shoulders, the neck, pegbox and scroll the rest. The lower bout
  is about 0.35 of the full height across, the upper bout about 0.83 of the lower bout, the waist about 0.72.
  The model makes the body a little wider (0.40 of the height) so the waist still reads at ball size.
- **Body.** Double-bass shaped, not cello shaped: sloping shoulders running into the neck heel, short C-bouts, an
  arched front with a dark edge, deep ribs. Two f-holes either side of the waist, the bridge standing between their
  notches; a large dark ebony tailpiece below it.
- **Neck and scroll.** A very long ebony fingerboard from just above the f-holes to the nut, the neck under it
  leaving the body at the shoulders; an open pegbox with big tuning keys (two on one side, one on the other for the
  three strings) and a carved scroll on top.
- **Strings.** Three, pale gut, very thick, running from the tailpiece over the bridge to the nut.
- **Levers.** The player can't reach the strings with the left hand, so a row of metal stop levers sits on the upper
  end of the fingerboard (seven positions; they press the strings onto fixed stops), worked through long linkage
  rods running down beside the neck from hand levers that stick out sideways at the left shoulder (brass on the
  Phoenix instrument), and from foot pedals at the base. On the photos the mechanism is on the left of the
  instrument (the player's side).
- **Base.** It stands on its endpin on a dark plinth, with a step for the player and a pedal or lever box beside it.
- **Colours.** The varnish runs from orange-brown (Montreal, Phoenix) to red-brown (Paris); the fighter's is deep
  umber rgb(100, 64, 36) with dark edges rgb(52, 32, 16) and lighter varnish streaks and gloss down the middle of the
  front. Ebony black for the fingerboard and tailpiece, pale maple for the bridge, pale strings, brass and steel for
  the mechanism.

## Model and previews

`studio/scripts/models/octobass_model.py` builds the instrument upright and tips it back 8 degrees towards the camera
so the front plate catches the light; the mechanism is on its -x side, which turns towards the camera at the preview
yaw of 30 (keep the look's yaw near the front, turned the same way). `render_r100_yaw30.png` is radius 100 px at x2,
`render_r40_yaw30_x3.png` radius 40 px at x3, both drawn with the series rasteriser
(`series/instrument-history/piano_lowpoly.py`: `render_geo` + `outline`) through the sprite camera of
`projects/which_instrument_wins/sprites.py` (elevation 16 degrees), exactly as SNARE DRUM's previews were (that set-up
reproduces SNARE DRUM's committed previews pixel for pixel).

The lever row is drawn larger than life on purpose: seven steel stops across the upper fingerboard with brass arms
sticking out to the side, three linkage rods down to a fan of three brass hand levers at the shoulder, and three brass
foot pedals at the front of the plinth, so the signature reads at ball size.

One refinement after comparing with the photos: the first pass had a cello-deep waist and a too-round upper bout (it
read as a guitar); the waist was widened and the C-bouts shortened to the bass outline of the Phoenix photo, and the
patchy wood colours were calmed to one varnish streak with gloss on the bouts.
