# BALLFIGHT Studio: interface reference

Everything a worker needs to build the app's interface without opening the two design artifacts. Extracted on
Nov 10, 2026 from:

- the approved mock-ups, five screens at 1440x900 (https://claude.ai/artifact/DWR46mnrEU6mTxyt6iXBrW), and
- the owner's Ball Studio design system (https://claude.ai/artifact/SPfnA1YGCJr8SHFstvjxji).

This file is design data. Where it and the design spec (`2026-11-10-ballfight-studio-design.md`, section 11) disagree,
the spec wins. Three things in the mock-ups are **stand-ins, not requirements**: the preview pictures are drawings,
every number is sample data, and the fighter names are bitmap images of the pixel font. Section 9 lists where the app
departs from the mock-ups on purpose.

All sizes are in points at `pixels_per_point = 1` (one point = one CSS pixel of the mock-ups).

## 1. Principles (from the design system)

- **The video owns colour.** Panels, controls and text are neutral greys with no hue. Never tint a panel, a button or
  a slider with an accent.
- **Colour belongs to fighters.** Fighter colours take the place of Ball Studio's "track colours". A colour always
  means "this fighter" and never a state or an error, so errors are stated in words.
- **Dense and precise.** Controls are 26 tall, corners 2, lines 1. Many settings fit without scrolling.
- **One way to change a value.** Sliders and value fields change values. Nothing else is clickable inside a row.
- **The inverted fill is the only accent.** A button that is on, and the primary action, fill with `text` and set
  their label in `on-light`.
- Sentence case everywhere. Name things by what they do in the video, never by code names. A button label says
  exactly what happens; it ends with "…" only when it opens a window.
- Panels touch with 1 px `line-soft` dividers. No gaps, no shadows (menus excepted), no rounded panels.

## 2. Tokens

### Colours

| Token | Hex | Use |
|---|---|---|
| `ground` | `#1e1e1e` | Window background, status bar, text and value field wells |
| `panel` | `#282828` | Toolbar, bout bar, workspaces, side panels, timeline. Base that fighter fills are mixed over |
| `stage` | `#2b2b2b` | The column behind the 9:16 preview |
| `raised` | `#333333` | Dropdowns, small ("box") buttons, menus |
| `selected` | `#383838` | Fill of a selected row or chip, always with a 1.5 px `text` ring |
| `hover` | `#3a3a3a` | Hover fill of flat buttons and of menu items that are not highlighted |
| `line-soft` | `#383838` | Dividers between panels and sections |
| `line` | `#444444` | Control outlines, field borders, slider tracks, the ribbon's centre line |
| `faint` | `#777777` | Disabled text, the outline and knob of a switch that is off. Never for text that must be read |
| `muted` | `#a0a0a0` | Labels, hints, counts, lane names, the status bar |
| `text` | `#e2e2e2` | Values, names, body text. Also the fill of on and primary buttons |
| `text-strong` | `#ffffff` | Playhead, K.O. bar, slider knob, text of a selected chip, the failed-job headline |
| `on-light` | `#1e1e1e` | Text on a `text` fill |

Extra greys the mock-ups use: `#303030` (the hairline under each queue job and each arrangement lane), `#2e2e2e` (the
group header lanes "Fighters" and "Band"), `#666666` (histogram bars outside the K.O. window), `#3a3a3a` with a 1 px
`muted` top edge (band clips on the Music screen).

Menu shadow: `0 8px 20px #000000aa`. Nothing else casts a shadow. Focus ring: 2 px `text-strong`, 2 px outside.

### Fighter colours

Full colour for small marks (4 px bars, ribbon fills, ability ticks, the ring around a portrait). **Large fills** use
the colour mixed 22 % over `panel` (per channel `round(40 + 0.22 * (c - 40))`) with a 1 px edge in the full colour.
Checked against the mock-ups: FLUTE `#dee8f6` gives `#505255`, TROMBONE `#78f0b8` gives `#3a5448`.

The roster as drawn (sample data; the app reads names and colours from the engine's roster, and the order of the
groups and of the fighters inside each is the order below):

| Section | Fighters and colours |
|---|---|
| Woodwinds | Flute `#dee8f6`, Oboe `#9cb034`, Clarinet `#bea8ff`, Bass clarinet `#6e84aa`, Bassoon `#1e824c`, Alto sax `#c850f0`, Tenor sax `#e66e1e`, Bari sax `#ff765c`, Bagpipes `#40c458` |
| Brass | Trumpet `#ffd63c`, Horn `#c87040`, Trombone `#78f0b8`, Euphonium `#f0ac20`, Tuba `#685cec`, Sousaphone `#d21e3c` |
| Strings | Violin `#b062ff`, Viola `#24c8b0`, Cello `#c45428`, Contrabass `#466ec8`, Harp `#f4e4be`, Acoustic `#d6a064`, Electric `#64ff78`, Ukulele `#ff8ab2` |
| Keys and electronics | Piano `#549cff`, Organ `#c43468`, Accordion `#ec52d6`, Synth `#46e4ff`, Turntables `#bef53c` |
| Percussion | Drums `#ff9228`, Bass drum `#80808c` |

GUITAR and SAX are retired and never appear anywhere.

### Type

IBM Plex Sans for everything, IBM Plex Mono for numbers that must line up. Both are built into the exe.

| Style | Font | Size / line | Weight | Use |
|---|---|---|---|---|
| `title` | Sans | 18 / 24 | 600 | The four big numbers on Balance. Window titles |
| `section` | Sans | 13 / 18 | 600 | Section headings ("Roster", "Filters", "Queue"), the app name, job titles |
| `body` | Sans | 13 / 18 | 400 | Buttons, labels, rows, menus. The default |
| `small` | Sans | 12 / 16 | 400 | Hints, counts, lane names, column headers, the status bar |
| `value` | Mono | 12 / 16 | 400 | Numbers in value fields, seeds, scores, K.O. times, chord loops |
| `timecode` | Mono | 14 / 18 | 400 | The transport's time |

Sans Medium (500) is used only for the text of a selected chip or row.

### Spacing, corners, sizes

| Token | Value | Use |
|---|---|---|
| `space-1` | 2 | Gap between toolbar buttons |
| `space-2` | 4 | Gap between small controls in one row |
| `space-3` | 6 | Vertical item spacing; rows in a roster group are 3 apart |
| `space-4` | 8 | Gap between a label, its slider and its value |
| `space-5` | 10 | Side padding of toolbar, transport, status bar, rows |
| `space-6` | 16 | Gap between unrelated groups |
| `radius-sharp` | 2 | Buttons, fields, dropdowns, menus, switches |
| `radius-row` | 8 | Roster chips and shortlist rows only, so they read as objects |
| `radius-none` | 0 | Panels, lanes, clips, the preview |
| `control-h` | 26 | Buttons, fields, setting rows |
| `value-h` | 22 | Value fields and dropdowns inside a row |
| `row-h` | 30 | A roster chip |
| `toolbar-h` | 40 | Toolbar |
| `status-h` | 24 | Status bar |
| `label-w` | 108 | Label column of a setting row |
| `value-w` | 62 | Value field column |
| `lane-name-w` | 120 | Lane name column of the story timeline |

## 3. Controls

- **Flat button** (toolbar, transport): 26 tall, padding 0 x 10, no outline, transparent until hovered (`hover`).
  Text `text`.
- **Box button** (small actions: "Swap sides", "Stop search", "Change folder", "Remove"): as flat, on `raised` with a
  1 px `line` border.
- **On / primary button**: `text` fill, `on-light` label. Marks a toggle that is on (the current step, Loop, a
  chosen segment) and the screen's main action ("Balance this match-up", "Auto-balance", "Search seeds", "Use seed
  18433", "Go to render", "Add to queue").
- **Disabled**: keeps its place, text `faint`, no hover fill.
- **Segmented control**: buttons 24 tall sharing a 1 px `line` border, no gaps, square inner corners; the chosen one
  is inverted.
- **Slider**: a 2 px `line` track, a `muted` fill from the left to the knob, and a 3 x 14 `text-strong` bar as the
  knob. Never coloured.
- **Value field**: a `ground` well, 1 px `line` border, 22 tall, mono text right-aligned with its unit ("133.77 bpm",
  "×2.00", "49.0 s"). Drag sideways or type to change it.
- **Text field**: a `ground` well, 1 px `line` border, 26 tall (the caption box is three lines). A placeholder is
  `faint`.
- **Dropdown**: `raised`, 1 px `line` border, the value on the left and a 10 x 10 chevron (`muted`, 1.5 stroke) on the
  right. Its list is a menu.
- **Menu**: `raised`, 1 px `line` border, corner 2, padding 3, the shadow above; items 24 tall, padding 0 x 8; the
  hovered item inverts; a divider is a 1 px `line` with 3 above and below.
- **Switch**: a square 30 x 16 outline (1 px, corner 2) with a 10 x 10 square knob, 2 from the edge; `text` when on
  (knob right), `faint` when off (knob left). The mock-up draws it 26 x 14; use the design system's 30 x 16.
- **Chip / object row** (roster chips, shortlist rows): corner 8. Selected: `selected` fill, 1.5 px `text` ring drawn
  inside, text `text-strong`.
- **Progress bar**: 4 tall, `line` track, `text` fill.
- **Fighter bar**: a 4 px wide bar in the fighter's colour, corner 1: 24 tall in the bout bar, 18 in rows, 16 in queue
  jobs, 34 beside a personality, 30 as the left edge of a roster chip.
- **Icons** are painted, never glyphs. On a 12 x 12 box: play = triangle (2,1) (11,6) (2,11); pause = two 3 x 10 bars
  at x 2 and x 7, y 1; done = a 1.6 px check (2,6.5) (5,9.5) (10,3); now = a 1.6 px ring of radius 3 at (6,6); to
  do = a 1.6 px dash from (2,6) to (10,6). Chevron on 10 x 10: (2,3.5) (5,6.5) (8,3.5).

## 4. The frame (every screen)

```
+--------------------------------- toolbar 40 --------------------------------------+
+--------------------------------- bout bar 48 -------------------------------------+
|  the step's workspace (fills the rest)                          | stage column 410 |
|                                                                 | 9:16 preview     |
+--------------------------------- story timeline 112 ------------------------------+
+--------------------------------- status bar 24 -----------------------------------+
```

Reference window 1440 x 900, so the body is 676 tall and the workspace 1030 wide. Minimum window 1280 x 800 (body
576 tall, workspace 870 wide).

**Toolbar** (`panel`, 1 px `line-soft` below, padding 0 x 10, gap 2): "BALLFIGHT Studio" in `section` style with 16
after it; five flat buttons "1 Match-up", "2 Balance", "3 Seeds", "4 Music", "5 Render", the current one inverted; a
spacer; "Library", "Queue" followed by the number of queued videos in mono `muted` ("Queue 2"), "Update".

**Bout bar** (`panel`, 1 px `line-soft` below, padding 0 x 16, gap 14): left fighter's bar (4 x 24), the left name in
the pixel font 21 tall, "VS" in the pixel font 14 tall at 60 % opacity, the right name 21 tall, the right fighter's
bar; a spacer; then label and value pairs (label `muted`, 8 between label and value, 28 between pairs): "Theme" "Big
top"; "Tempo" "133.77 bpm" (mono); "Handicap" "×2.00 / ×0.40" (mono; drawn on Seeds, Music and Render); "Seed" then
"not picked yet" (`muted`) or the seed number (mono).

**Stage column** (410 wide, `stage`, 1 px `line-soft` on its left): the preview centred, 8 below it one `small`
`muted` line. Match-up and Balance: the VS title card, "Title card, also saved as the cover image". Seeds and Music:
the fight, "Seed 18433, the real picture and sound". Render: "Flute vs Trombone, seed 18433". (The mock-ups draw the
picture 351 x 624; see section 9.)

**Story timeline** (112 tall, `panel`, 1 px `line-soft` above). A transport row 30 tall (padding 0 x 10, 1 px
`line-soft` below): play or pause button (12 x 12 icon, padding 0 x 6), the time "0:31.40 / 1:01.00" in `timecode`
style, a spacer, on Seeds the hint "Drag anywhere on the story to scrub" (`small` `muted`), then "Loop" and "Safe
zone" toggles. Under it the story, 81 tall:

- a lane-name column 120 wide with a 1 px `line-soft` on its right, `small` `muted`: the left fighter's name (20
  tall), "Who leads" (41 tall), the right fighter's name (20 tall);
- the lanes, as wide as what is left (1320 at the reference size). Time runs left to right, 0 to 61 s. Top lane (20):
  the left fighter's first use of each ability, each a 2 px bar in its colour, 14 tall from y 3, with the ability's
  name to its right in its colour (`small`, 5 after the bar). Middle (41): the **lead ribbon** (below). Bottom lane
  (20): the same for the right fighter.
- the playhead: a 1 px `text-strong` line across all three lanes.
- on Seeds the words "K.O. window" (`small` `muted`) just right of the shaded window.

Before a seed is picked the story area holds one centred `muted` line, "The fight's story appears here once you pick
a seed.", the time reads "0:00.00 / 1:01.00" in `muted`, and the play button, Loop and Safe zone are `faint`
(disabled).

**The lead ribbon** (41 tall on the timeline, 28 tall and up to 400 wide in a shortlist row): a 1 px `line` centre
line; the K.O. window, 49 to 53 s, shaded `raised` over the full height (timeline only); the left fighter's colour
filled between the centre line and the lead curve **above** the line while it leads, the right fighter's colour
**below** while it leads; a 2 px `text-strong` bar at the K.O., inset 2 from top and bottom. The curve starts on the
centre line at FIGHT!, has one point per hit at `(time, hp_left - hp_right)`, straight lines between points, and
returns to the centre line at the K.O. Height: `min(|lead| * (mid - 1) / 42, mid - 1)` where `mid` is half the
ribbon's height, so a 42 HP lead fills a side.

**Status bar** (24 tall, `ground`, `small` `muted`, padding 0 x 10): what the app is doing or last did on the left;
on the right "30 fighters, 26 themes" (the counts the app really has).

## 5. The five screens

Wording below is the mock-ups' own, with their sample numbers. Text in quotes is exact.

### 5.1 Match-up

Three columns left of the stage: left corner card 250, the roster in the middle (530 at the reference size), right
corner card 250. All `panel`, 1 px `line-soft` between.

**Corner card** (padding 14 x 12, gap 14): "Left corner" (`small` `muted`; "Right corner" on the other); a row with
the fighter's portrait (a 52 round disc with a 3 px ring in its colour) and its name in the pixel font, 28 tall when
it fits and 21 when it does not (gap 12); the weapon's name in `section` style with a `muted` paragraph under it; a
divider, then "Abilities" (`section`) and three rows, each a 30 wide mono `muted` tag ("LV2", "LV3", "LOW"), the
ability's name and a `small` `muted` line under it; a divider, then three rows with a 92 wide `muted` label: "Range",
"Usually plays", "Colour" (a 12 x 12 swatch, corner 2, then the colour's name).

| | Left: FLUTE | Right: TROMBONE |
|---|---|---|
| Weapon | "Snake charmer" | "Slide lance" |
| Text | "Cobras rise from dropped baskets and strike on the beat. Every hit that lands lets them reach further." | "The slide shoots out at the enemy on the beat, then pulls back. Every hit that lands makes it longer. It can parry." |
| LV2 | "Flutter tongue", "Twice as fast for 8 beats" | "Sad trombone", "A ring that hits and slows" |
| LV3 | "Snake dance", "Lasts 4 beats" | "Power slide", "Twice as fast for 8 beats" |
| LOW | "Piccolo", "A wave across the ring, below 25 HP" | "Pedal tone", "A heavy slam, below 25 HP" |
| Range | "C5 to A6" | "E2 to F4" |
| Usually plays | "Melody" | "Bass" |
| Colour | "Silver" | "Mint" |

**Roster** (padding 14 x 16, gap 10): a header row with "Roster" (`section`), "30 fighters" (`small` `muted`), a
spacer and "Click for the left corner, right-click for the right" (`small` `muted`). Then one block per section: its
name (`small` `muted`), 4 under it a grid of chips, four columns, 6 between columns and 3 between rows. A chip is 30
tall, corner 8: a 4 x 30 bar in the fighter's colour at its left edge, a 14 round disc (2 px ring in the colour,
filled with the colour at one third opacity), the name, and at the right "Left" or "Right" (`small` `muted`) for the
two chosen fighters. A chosen chip has the selected look.

**Bottom strip** (1 px `line-soft` above, padding 12 x 16, gap 8), two rows:

- "Theme" (`muted`, 108 wide), a 220 wide dropdown "Big top", then `small` `muted` "A sinister circus in a minor
  key. Leaves the bass and the top line to the fighters."
- "Tempo" (`muted`, 108 wide), a slider at most 220 wide, an 86 wide value field "133.77 bpm", `small` `muted` "34
  bars fit in 61 seconds", a spacer, box button "Swap sides", primary button "Balance this match-up".

Status bar: "Flute vs Trombone, not balanced yet".

### 5.2 Balance

One column 780 wide, centred in the workspace, padding 28 above and below, 22 between blocks.

1. **Win split.** A row: "Flute wins 47%" (`section`) at the left, "3,000 fights, on seeds the search never uses"
   (`small` `muted`) in the middle, "Trombone wins 53%" (`section`) at the right. Under it (8) a bar 30 tall split at
   the left fighter's share: each part filled with its fighter's large-fill colour with a 2 px top edge in the full
   colour. Two 1 px `text-strong` marks at 40 % and 60 % reach 4 above and below the bar. Under it (8) a 16 tall row:
   "40%" and "60%" (`small` `muted`) centred on the marks and "In range" (`small`, `text`) centred at 50 %.
2. **Damage handicap** (1 px `line-soft` above, 16 padding): a row with "Damage handicap" (`section`), box button
   "Reset both to ×1.00", primary button "Auto-balance". Two rows 26 tall: the fighter's bar (4 x 18), "Flute damage"
   (`muted`, 120 wide), a slider, a 62 wide value field "×2.00". Then `small` `muted`: "Damage is the only thing you
   can change here. Reach, timing and abilities stay exactly as each fighter was built."
3. **Four numbers** in one box (1 px `line-soft` border, corner 2, cells divided by 1 px `line-soft`, each padded 10
   x 12): a `small` `muted` title, the number in `title` style, a `small` `muted` note.
   "Typical K.O." "50.6 s" "Most land between 44.2 and 56.9 s"; "Lead changes" "2.6" "On average per fight"; "No
   K.O. by 58 s" "4%" "These never reach the shortlist"; "Fit the beat sheet" "8%" "About 1 fight in 12".
4. **When the K.O. lands**: a row with that title (`section`) and `small` `muted` "Lighter bars are inside the beat
   sheet's window, 49 to 53 s"; a histogram 120 tall with a 1 px `line` under it, 28 bars for the seconds 30 to 57,
   2 apart, `text` for seconds 49 to 52 and `#666666` for the rest; under it mono `muted` labels spread evenly: "30
   s", "37 s", "44 s", "51 s", "58 s".
5. At the bottom, right-aligned: `small` `muted` "Changing the handicap gives a new set of fights, so the search
   starts over." and primary button "Search seeds".

Status bar: "Balanced on 3,000 fights in 1.4 s".

### 5.3 Seeds

**Filters** (230 wide, padding 14 x 12, gap 14, 1 px `line-soft` on its right): "Filters" (`section`), then groups,
each a `muted` label with its control 5 under it:

- "Winner": segments "Any", "Flute", "Trombone".
- "K.O. lands between": value field "49.0 s", the word "and" (`muted`), value field "53.0 s".
- "Lead changes": "At least" (`muted`) and a 62 wide value field "2".
- "Finish": segments "Any", "Close", "Comeback".
- "This ability lands": a dropdown "Any ability".
- a divider, then a switch with "Show fights outside the beat sheet".
- at the bottom, after a divider: "Watch any seed", a text field with the placeholder "Seed number" and a box button
  "Watch".

**Shortlist.** A header row 30 tall (`small` `muted`, padding 0 x 14, gap 12, 1 px `line-soft` below): a 30 wide
rank column with no title, "Seed" (62), "Who leads" followed by an 8 x 8 swatch and "Flute above", another swatch and
"Trombone below" (400), "Winner" (150), "K.O." (58, right-aligned), "Changes" (58, right-aligned), "Story" (the rest,
right-aligned). Rows 38 tall, corner 8, padding 0 x 10, gap 12, the list padded 2 x 4: rank (mono `muted`), seed
(mono), the row's lead ribbon (28 tall), the winner (a 4 x 18 bar in its colour, its name, then `small` `muted` "16
HP left"), the K.O. time (mono, "49.1 s"), lead changes (mono), the story score (mono, two decimals). The chosen row
has the selected look. A footer 46 tall (1 px `line-soft` above, padding 0 x 14): `muted` "212 fights fit the beat
sheet, from 18,240 checked. Still searching.", box button "Stop search", a spacer, primary button "Use seed 18433".

Status bar: "Searching on 7 cores".

### 5.4 Music

**Controls** (300 wide, padding 14 x 12, gap 13, 1 px `line-soft` on its right), groups as on Seeds:

- "Theme": a full-width dropdown "Big top".
- "Key": a 70 wide dropdown "E", then `muted` "harmonic minor, from the theme".
- "Chord loop": one button per loop, 28 tall, mono, left-aligned ("Em  Am  B7  Em  C  Am  B7  B7", "Em  C  Am
  B7"); the chosen one has the selected look, the others are box buttons.
- "Who plays bass": segments "Flute", "Trombone"; then `small` `muted` "The other fighter takes the melody."
- a divider, "This take": a row with "Music seed" (`muted`), a 62 wide value field "0417" and box button "Re-roll
  music"; then `small` `muted` "A new take writes fresh lines and solos. The fight doesn't change."
- a divider, then one block per fighter: its bar (4 x 34), a `section` line ("Flute, the charmer", "Trombone, the
  slide") and a `small` `muted` line ("High and ornamented. Runs of eighths that fall back to the home note.",
  "Smears up into notes, oom-pah struts, low fanfares.").
- at the bottom `small` `muted`: "Tempo is set on Match-up, because it changes the fight."

**Arrangement.** A header 40 tall (padding 0 x 14): "Arrangement" (`section`) and `small` `muted` "How this take
follows the fight you picked". Then lanes with a 112 wide name column (`small` `muted`, padding 4 x 10) and a 1 px
`#303030` line under each:

- a marker lane 22 tall: each marker a 1 px `faint` line with its `small` `muted` name 4 to the right: "Title card",
  "Fight", "Lead change", "Desperation", "K.O.", "Reset";
- a group header 22 tall on `#2e2e2e`: "Fighters" (`section`);
- one lane 25 tall per fighter, with clips (from y 2, 20 tall, text padded 1 x 5, `small`) in the fighter's large
  fill with a 1 px top edge in its colour: "Melody", "Answering line", "Melody, fuller", "Solo", "Bass", "Bass and
  melody", "Bass, thinner";
- a group header "Band", then lanes "Drums", "Calliope", "Organ", "Glockenspiel", "Band hits" with clips in
  `#3a3a3a` under a 1 px `muted` top edge ("Hits", "Bass drum and cymbal", "March snare", "Victory", "Build",
  "Off-beat chords", "Held chord", "Glints"; band hits are 6 wide ticks).

Under the lanes (padding 16 x 14, gap 6): "What the music listens for" (`section`) and rows with a 150 wide `muted`
label: "A lead change" "A drum fill, and the new leader picks up the melody."; "Who is ahead" "The leader's part
gets fuller. The other's thins and roughens as its HP drops."; "A desperation move" "The band drops out to a
breakdown for two bars."; "The final exchange" "Every layer comes in."; "Each ability" "One hit from the whole
band."; then `small` `muted` "Loudness stays level from the first frame. The build comes from adding parts, never
from turning it up." A footer 46 tall (1 px `line-soft` above): `muted` "Sounds right?", a spacer, primary button "Go
to render".

Status bar: "Soundtrack rebuilt for take 0417".

### 5.5 Render

**This fight** (padding 14, gap 10, 1 px `line-soft` below): a row "This fight" (`section`) and `small` `muted`
"Seed 18433, music take 0417". Then rows 6 apart:

- "Caption" (`muted`, 70 wide), segments "Instagram", "TikTok", "Shorts" (270 wide), `small` `muted` "Drafted for
  you. Edit freely."
- starting 78 from the left, `small` `muted` "Text and tags for Instagram" and under it (4) a three-line text box
  (padding 6 x 8) holding the caption, a line break, and the tags.
- "Saves to" (`muted`, 70 wide), a value field as wide as the row holding the folder
  ("Videos\BALLFIGHT\flute_vs_trombone\"), box button "Change folder".
- starting 78 from the left, `small` `muted` "flute_vs_trombone.mp4, cover.png, caption.txt and the fight file", a
  spacer, primary button "Add to queue".

**Queue.** A header 40 tall (padding 0 x 14, 1 px `line-soft` below): "Queue" (`section`), `small` `muted` "1
rendering, 1 waiting", a spacer, box button "Pause queue". Then one row per video (padding 8 x 14, gap 12, 1 px
`#303030` below): a 27 x 48 thumbnail (the two fighters' colours split on a diagonal), then a title row (the left
fighter's bar 4 x 16, "Tuba vs Bass drum" in `section` style, the right fighter's bar, `small` `muted` "Dark
drumline, seed 9120"), then by state:

| State | Under the title | Buttons |
|---|---|---|
| Rendering (row has the selected look) | a progress bar and, 150 wide and right-aligned in mono, "Drawing frames 64%" | "Cancel" |
| Waiting | `muted` "Waiting" | "Remove" |
| Stopped by a check | `text-strong` "Stopped at the loudness check", then `muted` "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was saved. Try another music take, then queue it again." | "Open in Music", "Remove" |
| Done | `muted` "Done in 48 s. All checks passed." | "Open folder" |

**The job's check list** (330 wide, padding 14 x 12, gap 4, 1 px `line-soft` on its left): the video's name
(`section`), `small` `muted` "Every video goes through these in order. A failed check stops that video and the queue
moves on." (8 under it), then ten rows at least 26 tall, each an icon (done, now, to do) and a name with a `small`
`muted` note under it. A row to do is `muted` with a `faint` icon; the row in progress is `text-strong`; done rows
are `text`.

| Row | Note |
|---|---|
| "Soundtrack built" | |
| "Safe zone" | "Captions and HUD clear of the apps’ buttons" (the mock-up's own curly apostrophe) |
| "Focus" | "Both fighters sharp on every frame" |
| "Loudness" | "Opening within 0.6 dB of the ending" |
| "Fighters above the music" | |
| "Flashes" | "Busiest stretch peaks at 17% of the frame" |
| "Loop point" | "The last frame flows into the first" |
| "Drawing frames" | "1,171 of 1,830" |
| "Encoding" | |
| "Checking the finished video" | "Length, loop and flashes again, on the mp4" |

Status bar: "Rendering Tuba vs Bass drum, about 20 s left".

## 6. The three fight touches (spec section 11)

1. The two names in the bout bar (and in the corner cards) are drawn with the series' own 5 x 7 pixel font
   (`reelkit/font.py`: glyphs 5 wide, 7 tall, advance 6) at whole-number scales: scale 3 (21 tall) in the bout bar,
   scale 2 for "VS", scale 4 (28 tall) in a corner card when the name fits its row, else 3.
2. The preview shows the VS title card on Match-up and Balance, since it is also the cover image.
3. The lead ribbon (section 4): shortlist rows are small ribbons, and the same ribbon is the scrub bar under the
   preview, with a lane above and below for each fighter's first use of each ability.

## 7. Keys

Space plays and pauses. Left and right arrows step one frame. 1 to 5 switch steps. None of them fires while a text
field has the keyboard.

## 8. Building it in egui

The design system's notes, as written:

- `Visuals`: `panel_fill` and `window_fill` = `panel`, `extreme_bg_color` = `ground`, `faint_bg_color` = `raised`,
  `window_stroke` = 1 px `line`, every `corner_radius` = 2 except chips and rows (8).
- Widget states: inactive fill transparent with `text` foreground; hovered fill `hover`; active and open fill `text`
  with `on-light` foreground; selection `bg_fill` = `text`.
- Fonts: load IBM Plex Sans (Regular, Medium, SemiBold) and IBM Plex Mono (Regular) TTFs with `FontDefinitions`;
  Proportional = Plex Sans, Monospace = Plex Mono.
- Spacing: `item_spacing` = 8 x 6, `button_padding` = 10 x 5, `interact_size.y` = 26.
- Large fill: the fighter colour linearly mixed 22 % over `panel`, with a 1 px edge in the full colour.

What was checked on this PC with the pinned crates (eframe, egui, egui-wgpu and egui_kittest 0.37.0, wgpu 30.0.1,
rustc 1.99.1, `x86_64-pc-windows-gnu`), so workers need not rediscover it:

- An app implements `eframe::App` with `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)` (required)
  and may implement `fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`, which eframe calls before
  every `ui` and also while the window is hidden if a repaint was requested. Work that must go on while the window
  is minimised (taking engine events, starting the next queued video) belongs in `logic`.
- Panels are `egui::Panel::top(id)`, `::bottom(id)`, `::left(id)`, `::right(id)` with `.exact_size(points)`,
  `.resizable(false)`, `.show_separator_line(false)`, `.frame(egui::Frame::new().fill(colour))` and `.show(ui, |ui|
  ...)`; the rest is `egui::CentralPanel::default().frame(...).show(ui, ...)`. They take a `&mut Ui`, not a context.
- Fonts: `FontDefinitions::empty()`, `font_data.insert(name, Arc::new(FontData::from_static(bytes)))`,
  `families.insert(FontFamily::Proportional | Monospace | Name("semibold".into()), vec![names...])`,
  `ctx.set_fonts(...)`. Variable fonts load at their default instance. With eframe's `default_fonts` feature off,
  egui brings no font of its own.
- A character no installed font has draws as a box in the app and **panics in tests** (the test harness sets
  `MissingGlyphPolicy::Panic`), which is the wanted guard against missing glyphs. egui's own emoji font lacks the
  flute, drum, accordion and harp, which is why the app carries its own (section 10).
- Styling for all themes at once: `ctx.all_styles_mut(|style| ...)`; text sizes through `style.text_styles`.
- A texture for the preview: `ctx.load_texture(name, egui::ColorImage::from_rgb([360, 640], &rgb),
  egui::TextureOptions::NEAREST)`, later `handle.set(image, egui::TextureOptions::NEAREST)`.
- Accessible names (the test harness fails any control without one): a `Button`, a `menu_button`, a `DragValue` and a
  `TextEdit` with `hint_text` are named by themselves; `ComboBox::from_id_salt` and `egui::Slider` are **not** and
  need `.labelled_by(label.id)` on their response; a control painted by hand gets its name and role from
  `response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, enabled, "name"))` (also
  `WidgetInfo::selected(Role::Switch, enabled, on, "name")` and `WidgetInfo::slider(enabled, value, "name")`).
- Keys: `ctx.input(|i| i.key_pressed(egui::Key::Space))`, `Key::ArrowLeft`, `Key::ArrowRight`, `Key::Num1` to
  `Key::Num5`; `ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))` takes a key away from the widgets;
  `ctx.egui_wants_keyboard_input()` is true while a text field has the keyboard.
- Small per-widget state (a texture made once, for instance) can live in egui's temporary data:
  `ctx.data_mut(|d| d.insert_temp(id, value))` and `d.get_temp::<T>(id)`. Rows can be dragged with
  `ui.dnd_drag_source` and `ui.dnd_drop_zone`; a right-click menu is `response.context_menu(|ui| ...)`.
- In a test, a control's state is read from its node: `node.accesskit_node().is_disabled()`,
  `.toggled() == Some(egui::accesskit::Toggled::True)`.
- Two wgpu test harnesses drawing at the same time crash the test process on Windows about every second run, so
  drawing tests share one lock and run one at a time.
- Tests: `egui_kittest::Harness::builder().with_size(egui::Vec2::new(1440.0, 900.0)).with_pixels_per_point(1.0)
  .with_render_cursor(false).wgpu().build_eframe(|cc| App::new(cc, ...))` draws the real app edge to edge
  (`build_ui` adds an 8 point margin and is for single widgets). `harness.run()` settles the interface;
  `harness.get_by_label("…")` finds a control (`use egui_kittest::kittest::Queryable as _`), `.click()`,
  `.click_secondary()`, `.rect()`; `harness.key_press(egui::Key::Space)`; `harness.unnamed_widgets()` lists controls
  without a name; `harness.state()` is the app; `harness.snapshot("name")` compares with
  `tests/snapshots/name.png`. `UPDATE_SNAPSHOTS=true` rewrites the images that differ. Settings live in a
  `kittest.toml` found from the crate folder upwards.
- The harness picks a software adapter by itself: on this PC "Microsoft Basic Render Driver" (Direct3D 12, type
  Cpu); on Linux it needs the `mesa-vulkan-drivers` package.

## 9. Where the app departs from the mock-ups

| Mock-up | App | Why |
|---|---|---|
| Preview drawn 351 x 624 | 360 x 640 at the reference size, at the largest whole-number scale of 360 x 640 that fits, else the largest that fits | Spec section 7: whole-number scale when the window allows |
| Names are images | Painted from the 5 x 7 glyphs | Touch 1 |
| A round disc as the portrait | The fighter's real sprite when the engine supplies one; the disc otherwise | The disc is a stand-in |
| Switch 26 x 14 | 30 x 16 | The design system's size |
| No place for pair rules | A third row in the Match-up bottom strip, "Pair rules", read-only | Spec section 4 |
| No Settings | A "Settings" button after "Update" in the toolbar, opening a view in the workspace | Spec section 4 |
| No Library screen | A view in the workspace: a grid of covers | Spec section 4 |
| Queue rows cannot be reordered | Drag a waiting row, or its right-click menu | Spec section 9 |
| "Open in Music" on every failed check | The step that can fix it: Music for a sound check, Seeds for a picture check | Wording rule: say what to do next |
| A failed loudness check advises "Try another music take, then queue it again." | "Open it in Music to see the levels, or try another seed, then queue it again." | New takes arrive with milestone 5; advice must be something the app can do |
| Right side of the status bar holds a note about sample data | The counts of fighters and themes the app has | The notes are the mock-ups' own |
| Arrangement clips carry names ("Melody", "Bass, thinner") | Until the music system is built (milestone 5): plain blocks showing where each part of the real soundtrack sounds | The names belong to the reacting arrangement, which does not exist yet |
| "What the music listens for" | Until milestone 5: "Loudness", the take's level checks, and one line saying what arrives later | Spec section 8: loudness is checked on the Music step; no claim the app cannot keep |
| Key, music seed and "Re-roll music" look alive | Until milestone 5: drawn disabled, each saying why | No dead buttons that look alive |
| "Update" looks alive | Until milestone 6: drawn disabled, saying updates arrive later | The same |
| Two chord loops; either fighter can take the bass | What the content really has: one loop for Big top; the flute has no bass part | The mock-up's are sample data |
| All 30 fighters can be picked | The fighters built so far can; the rest are drawn disabled and say they arrive in an update | Milestones 3 and 4 build them |
| "Stop search" only | "Stop search", then "Keep searching"; "The shortlist is full." at 200 fits | Spec section 6 |

## 10. Fonts in the exe

| File | From | Bytes | Git blob |
|---|---|---|---|
| `IBMPlexSans-Regular.ttf` | `IBM/plex`, tag `@ibm/plex-sans@1.1.0` (commit `1da12f02587b630c07e92692d21492d722f53614`), `packages/plex-sans/fonts/complete/ttf/` | 200,500 | `bd6817d5202895da5ac4fad88de3da71e652881a` |
| `IBMPlexSans-Medium.ttf` | same folder | 202,460 | `a3826c4e9ab3f2d6e64e0c9aea7122ea3431e174` |
| `IBMPlexSans-SemiBold.ttf` | same folder | 202,632 | `09ec8cdae8208c26165db0c87da409cda8c26ae4` |
| `IBMPlexMono-Regular.ttf` | `IBM/plex`, tag `@ibm/plex-mono@2.5.0` (commit `2f9ba1b25957d958db71a849e85d72e3ecfb845a`), `packages/plex-mono/fonts/complete/ttf/` | 173,052 | `4254c37f20ba57e829a3315fdc91cda4e63589f5` |
| `NotoEmoji[wght].ttf` | `google/fonts`, commit `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5`, `ofl/notoemoji/` | 1,982,596 | `c2c26ab612a88a8610ff9cfbb89299bf2aea6c7a` |

All five are under the SIL Open Font License 1.1, which allows bundling them in the app as long as the licence text
travels with them: `license.txt` beside the Plex files (4,360 bytes, blob `f72f76504cd73cf2c00b140b2743c138320597b6`)
and `OFL.txt` beside the Noto file (4,330 bytes, blob `d09d3d0e09169f6747754e6b25faf5ba5ef6402d`). `git hash-object
<file>` prints a file's blob id, which is how a download is checked. Noto Emoji is the monochrome emoji font; it is
there because captions are drafted with each fighter's emoji and the interface stays grey.

## 11. The mock-ups' sample data

Every number on the mock-ups is sample data. It is written down here so the app's sample data (`seam::sample`) and
every fixture show the same thing the owner approved. Times given in percent are percent of the 61 s timeline.

**The shortlist.** Fourteen seeds, 20418, 7731, 15290, 3306, 11874, 26051, 902, 18433, 5627, 23980, 14402, 8175, 29766
and 1248, each turned into a fight by the mock-ups' own script (ported in the plan's Task 8b) and ranked by score.
Run, it gives this order: 18433 (score 7.84, Trombone wins at 49.1 s with 16 HP left, no lead change), 5627 (7.75,
Flute, 50.3 s, 26 HP, 2 changes), 8175 (7.73, Trombone, 52.7 s, 4 HP, 3), 23980, 11874, 29766, 7731, 15290, 1248,
26051, 3306, 20418 (6.55, Trombone, 52.2 s, 10 HP, 4 changes), 14402, 902. The selected fight on every screen is
therefore **seed 18433**. Seven of the fourteen are won by Trombone; six end with the winner at 12 HP or less. The
footer's counts are 212 fits from 18,240 checked, searching on 7 cores. The mock-up's filter column reads "At least 2"
lead changes over rows that have fewer, so the sample engine does not filter by lead changes or by ability. First uses
of abilities, the same for every sample fight: Flute's "Flutter tongue" at 19.6 % and "Snake dance" at 50.2 %;
Trombone's "Sad trombone" at 27.9 %, "Power slide" at 42.6 % and "Pedal tone" at 67.2 %. FIGHT! is at 3.6 s. The
playhead stands at 0:31.40 (frame 942).

**Balance.** 3,000 fights; Flute wins 47 %; typical K.O. 50.6 s, most between 44.2 and 56.9 s; 2.6 lead changes on
average; 4 % with no K.O. by 58 s; 8 % fit the beat sheet (use a fit rate of 0.083, which reads "8%" and "About 1
fight in 12"); measured in 1.4 s. Handicaps ×2.00 and ×0.40. The histogram's 28 bars, for the seconds 30 to 57:
2, 3, 3, 5, 6, 8, 10, 13, 17, 22, 28, 36, 45, 56, 68, 80, 91, 98, 100, 96, 88, 76, 62, 47, 33, 21, 12, 6.

**Music.** Markers: "Title card" 0 %, "Fight" 5.9 %, "Lead change" 32.8 %, "Lead change" 54.1 %, "Desperation"
67.2 %, "K.O." 84 %, "Reset" 95.1 %. Lanes and where their clips run (start to end, in percent):

| Group | Lane | Clips |
|---|---|---|
| Fighters | Flute | 0 to 2.6; 5.9 to 32.5; 32.8 to 53.8; 54.1 to 83.7; 86 to 94.8 |
| Fighters | Trombone | 3 to 5.6; 5.9 to 32.5; 32.8 to 53.8; 54.1 to 83.7; 95.1 to 100 |
| Band | Drums | 0 to 5.6; 5.9 to 66.9; 72.1 to 83.7; 86 to 94.8; 95.1 to 100 |
| Band | Calliope | 5.9 to 66.9; 72.1 to 83.7 |
| Band | Organ | 54.1 to 66.9; 72.1 to 83.7 |
| Band | Glockenspiel | 72.1 to 83.7; 86 to 94.8 |
| Band | Band hits | half a percent wide, at 19.6, 27.9, 42.6, 50.2 and 67.2 |

The mock-up has no loudness lines (they replace "What the music listens for"); the sample uses three that pass:
"Loudness", "The opening is 0.6 dB quieter than the ending, and the limit is 1.5 dB."; "Perceived loudness", "The
title card is 0.4 LU under the fight."; "Fighters above the music", "Both fighters sit above the band in their own
range." The music seed reads "0417"; the two chord loops are `Em Am B7 Em C Am B7 B7` (chosen) and, drawn short on the
mock-up, `Em C Am B7` (use `Em C Am B7 Em C Am B7`).

**Captions.** The caption line is "The front row vs the back row 🪈🎺 Comment the next match-up 👇". Instagram's tags:
"#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying
#music". The mock-up shows only the Instagram tab; for the others the sample uses: TikTok, the same caption line and
"#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"; Shorts, three lines:
"Flute vs Trombone", "Flute vs Trombone, every hit in key. Comment the next match-up", "#shorts #ballfight #flute
#trombone #music #pixelart". The output folder reads `Videos\BALLFIGHT\flute_vs_trombone\`.

**The queue's four videos**, in the order drawn:

| Video | Colours | Theme, seed | State |
|---|---|---|---|
| Tuba vs Bass drum | `#685cec`, `#80808c` | "Dark drumline", 9120 | rendering: the first seven lines of the check list done, "Drawing frames" at 1,171 of 1,830 (64 %) |
| Alto sax vs Tenor sax | `#c850f0`, `#e66e1e` | "Dark disco", 15007 | waiting |
| Oboe vs Bassoon | `#9cb034`, `#1e824c` | "Heist", 662 | stopped at the loudness check: "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was saved." and then the advice of section 9 |
| Cello vs Contrabass | `#c45428`, `#466ec8` | "Dark tango", 4471 | done in 48 s |

The check list's notes while the first one renders: "Opening within 0.6 dB of the ending" under Loudness, "Busiest
stretch peaks at 17% of the frame" under Flashes, "1,171 of 1,830" under Drawing frames. The status bar reads
"Rendering Tuba vs Bass drum, about 20 s left"; the toolbar reads "Queue 2".

**The Library** has no mock-up. Its sample is six videos, newest first, each dated at 12:00 UTC so the date reads the
same in every time zone: Flute vs Trombone (seed 18433, Nov 10, 2026), Tuba vs Bass drum (9120, Nov 9), Cello vs
Contrabass (4471, Nov 8), Alto sax vs Tenor sax (15007, Nov 7; this one carries the problem "Made by a newer version
of the app."), Oboe vs Bassoon (662, Nov 6), Trumpet vs Horn (3306, Nov 5). Covers are drawn: the two fighters'
colours split on a diagonal, 180 x 320. The sample app version is "0.1.0".
