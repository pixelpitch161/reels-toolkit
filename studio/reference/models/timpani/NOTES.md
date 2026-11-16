# Timpani: modelling reference (low-poly ball fighter, seen from slightly above)

Photos in this folder (see sources.txt): 01 above view (Yamaha copper), 02 Ludwig set with pedals and struts,
03 labelled side drawing, 04/05 rim close-ups (crops), 06/07 mallets, 08 set of 4 from above.

## Overall shape
- One drum = a copper **kettle bowl** with a white **head** stretched on top, a chrome **counterhoop**, short
  **tension rods** around the rim, **struts** running down to a low **base**, and a **foot pedal** at the front.
- From slightly above (01, 08) the head is the biggest, brightest shape: a near-white disc ringed by a thin
  chrome band, with the copper bowl showing as a crescent below it.
- Total height to the head: about 0.9 to 1.0 x head diameter (head sits at about waist height, 75-85 cm).

## Bowl (kettle)
- Profile: a short near-vertical **collar** just under the rim (about 10-15% of bowl depth), then a smooth
  parabolic/near-hemispherical curve to a rounded bottom (02, 03, 05). Not a flat-bottomed pot.
- Depth to width: bowl depth about **0.55-0.65 x diameter** (deeper on the small drums, shallower on the big one).
- Bottom: rounded, with a small flat or hub at the centre where it meets the pedal mechanism (03).
- Low-poly: lathe of 16-24 sides, 5-6 profile rings; keep the collar as one straight ring under the rim.

## Copper colour and finish
- Warm polished copper, strongly reflective: orange highlights, deep red-brown shadows.
  Base about #B8642E, highlight about #F2A766, shadow about #5A2410.
- Two finishes: **hammered** (01, 04, 08: faceted patches, flicker in the light; great excuse for flat-shaded
  low-poly facets) or **smooth polished** (02, 05).

## Head
- Plastic head: warm off-white / cream, about #EDE6DA, matte to satin, slightly darker towards the edge.
  Small dark maker logo near the edge at the back (Remo/Yamaha marks in 01, 08). Calf heads are beige, mottled.
- The head lips over a rounded bowl edge; a thin pale ring of head is visible just inside the chrome hoop.

## Rim and counterhoop
- Counterhoop: bright **chrome** band, thin: about 2-3% of the diameter tall (roughly 1.5-2.5 cm on a 70 cm drum),
  sitting just above the bowl edge (04, 05). Under it a slightly darker band where the bowl collar starts.
- Model as one flat ring or low torus, 16-24 segments, light grey/white specular.

## Tension rods and lugs
- **8 rods** around the rim on modern pedal timpani (6 on very small drums), evenly spaced at 45 degrees.
- Each is a short vertical rod (about 1/12 of the diameter) from the hoop down to a small lug/claw on the bowl
  collar; on many drums the rod's lower end is hidden in the strut bracket (04, 05).
- Some drums (hand-tuned) show **T-handles** sticking up above the rim; pedal timpani usually have small square
  rod heads instead. Optional T-handles read well from above if the fighter needs "spikes".
- Low-poly: 8 small boxes on the hoop plus 8 thin box rods; chrome.

## Tuning gauge
- At the player-facing side of the rim: either a **fan-shaped indicator plate** below the rim with a pointer
  (03), or a vertical strip of coloured markers (red/white/black pegs) on a bracket next to the rim (01, 04),
  or a chrome pointer bracket on top of the hoop (02, 05).
- Size: about 1/6 of the diameter tall. Good spot for a glowing "power meter" detail.

## Struts
- **3-4 struts** (Ludwig and Yamaha styles) curving gently outward then down from rim lugs to the base (02, 03).
  Some models use 4-6 straight dark bars around the bowl (01, 04, 08).
- Flat bars, about 4% of the diameter wide; colour dark brown/black (01, 04, 08) or painted bronze/maroon (02).
- From above they barely show: just dark ticks beside the bowl.

## Base and pedal
- Base: a low, wide cast **ring or shallow pan** (about 0.5-0.6 x diameter), dark grey/black or bronze, on 3
  small casters (03; pans visible in 02).
- Pedal: a long **rectangular footplate** sticking out toward the player from the front of the base, angled
  up about 20-30 degrees; silver/aluminium with a diamond or ribbed texture (02, 03). Size about 0.35 x
  diameter long, 0.12 x diameter wide.
- A central spindle/rod runs from the base up to the bowl bottom (01 interior, 03).

## Mallets
- Shaft: straight, about 35-38 cm long, about 1 cm thick: pale natural wood (birch, bamboo or rattan with
  visible nodes) (06, 07).
- Head: felt ball about 3.5-4.5 cm, slightly flattened sphere or "cartwheel" disc; **white/cream felt** is
  standard (06, 08); also black (staccato) and red (07). Often a small coloured washer or ring (red, blue,
  green) where shaft meets head.
- Proportions: head diameter : shaft length about 1 : 9 to 1 : 10.
- Low-poly: 6-sided cylinder shaft, 8x6 sphere (squashed 85% on the shaft axis) for the head.

## Set of four (08, 02)
- Standard sizes about 32", 29", 26", 23" (81, 74, 66, 58 cm), arranged in an arc around the player.
  American setup: big drum on the player's left; German setup: big drum on the right.
- Heads sit at nearly the same height; the smaller drums have proportionally deeper bowls.

## Low-poly readability tips (seen from slightly above)
- The head disc + chrome ring + copper crescent is the instant read. Exaggerate the copper rim crescent and the
  chrome ring thickness slightly so they survive at small size.
- Hammered copper as flat-shaded facets gives sparkle for free.
- Silhouette accents that still read from above: rod heads/T-handles around the rim, the pedal pointing at the
  camera, the gauge on the front.

## The model (studio/scripts/models/timpani_model.py)

One 29-inch pedal drum, head diameter 1.0 before scaling: head 0.95 above the floor, 24 sides around, a straight
collar then a parabolic bowl (about 0.6 x diameter deep in all) split into flat triangles with small fixed dents and
shade changes for the hammered look, 8 rods at 22.5 + 45 k degrees, 3 struts at 30, 150 and 270 degrees (the pedal
and gauge sit at the front, +z, between two of them), a 12-sided base ring on 3 casters, a centre spindle, the ribbed
pedal and a fan gauge with ticks and a red pointer. The renders here come from a quick flat-shaded preview script,
not the game's sprite renderer.
