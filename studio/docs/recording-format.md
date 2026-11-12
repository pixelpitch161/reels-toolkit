# The recording file (`.bfrec`), format 1

A recording is one fight: who fought, under which rules, every event, and the state of the ring at every frame. The
Python kit writes recordings with `tools/studio_export/recording.py` (`write_bfrec`); the Studio reads and writes them
with `bf_core::recording`. This document is the contract between the two. If it and a file disagree, fix one of them.

The five reference recordings are in `studio/reference/fights/flute_vs_trombone/`: `ep392.bfrec` (the example
episode's own seed) and `seed1`, `seed2`, `seed3`, `seed500`. The scenario recordings and the fight with no K.O.
(`no_ko.bfrec`) are listed in section 7.

## 1. The container

A `.bfrec` file is a zip archive with these members and no others:

| Member | What it holds |
| --- | --- |
| `meta.json` | every scalar field, the events, and the list of arrays (section 2) |
| `<KEY>.npy` | one rectangular numeric per-frame array (section 4) |
| `<KEY>.json` | one ragged per-frame array (section 5) |

`<KEY>` is an array's name: ASCII letters, digits and `_` (`P`, `HP`, `WAVES`, `ECHO_G`). It is never `meta`.

Zip details:

- Each member is stored (method 0) or deflated (method 8). A reader accepts both. No encryption, no zip64, no data
  descriptors, no comments, no folders.
- A reader finds members by name through the central directory. Their order means nothing.
- Python writes the members in ascending name order (byte order, so `meta.json` is last), deflated at level 9, with
  the timestamp 1980-01-01 00:00:00 on every member, `create_system = 0` and `external_attr = 0o600 << 16` (the
  last two matter only to reproduce Python's bytes), so the same fight always gives the same file bytes. A Studio
  writer should do the same (sorted names, one fixed timestamp, one compression setting). The bytes of the zip are
  not part of the content hash (section 7), so a different deflate implementation does not change a recording's
  identity.

A reader must refuse, with an error that names the file and the problem and without panicking:

- a file that is not a complete zip (a truncated file has no central directory),
- a zip without `meta.json`, or whose `meta.json` is not the JSON described below,
- `format` greater than the reader knows (the Studio says: `format 2 ... this app reads 1`), or below 1,
- a `meta.json` with a key this document does not list, or with the same key twice in one object,
- an array named in `meta.json` whose member is missing, or a member that `meta.json` does not name,
- a `.npy` whose header is not as in section 4, or whose data is shorter or longer than its shape says,
- arrays that disagree on the number of frames (section 3), or a recording without `HP`.

## 2. `meta.json`

One JSON object, UTF-8. Python writes it with sorted keys, one event per line, and escapes non-ASCII characters; a
reader must not depend on the layout or the key order.

| Key | JSON type | Rust field | Meaning |
| --- | --- | --- | --- |
| `format` | integer | `format: u32` | `1`. The reader's first check. |
| `source` | object | `source: Source` | Who made the recording (below). |
| `names` | list of strings | `names: Vec<String>` | The fighters, left corner first: `["FLUTE", "TROMBONE"]`. Everything indexed "per fighter" uses this order; index 0 is the left (red) corner. |
| `seed` | integer, 0 to 2^63 - 1 | `seed: u64` | The fight's seed. |
| `winner` | integer or `null` | `winner: Option<usize>` | Index into `names` of the winner; `null` when nobody was knocked out before `rules.t_max`. |
| `t_ko` | float or `null` | `t_ko: Option<f64>` | Time of the K.O. in seconds on the video clock; `null` exactly when `winner` is `null`. |
| `f0` | integer | `f0: usize` | The video frame that recording frame 0 belongs to: `round(rules.t_fight * fps)` (108 in the reference fights). |
| `fps` | integer | `fps: u32` | Recorded frames per second (30). |
| `corner` | list of `[x, y]` floats, one per fighter | `corner: Vec<[f64; 2]>` | Where each fighter starts, in ring units: `[[-1.85, 1.85], [1.85, -1.85]]`. |
| `rules` | object | `rules: BTreeMap<String, Value>` | The rules the fight ran under (section 6.1). |
| `events` | list of lists | `events: Vec<Event>` | Every event in time order (section 6.2). An event is `[t, kind, who, ...]`: `t` a float, `kind` a string, `who` an integer, then the kind's own fields. `Event.t` is element 0, `Event.kind` element 1, and `Event.fields` is everything from element 2 on, so `Event::at(2)` is `who`. |
| `arrays` | object | (drives `arrays: BTreeMap<String, Array>`) | Which per-frame arrays the file has and how each is stored: `{"P": "npy", "WAVES": "json", ...}`. `"npy"` means the member `<KEY>.npy` (`Array::F64` or `Array::I64`), `"json"` means `<KEY>.json` (`Array::Nested`). This is how a reader learns which keys exist and of which kind; it then checks that the zip holds exactly these members plus `meta.json`. |

Times are seconds on the video clock: `t = 0` is the first frame of the video and the fight starts at
`rules.t_fight`. Positions are in ring units in the ring's own top-down frame: the ropes are at `|x| = A` and
`|y| = A` (`A = 3.0`), the left (red) corner is at `(-A, +A)`, the right (blue) corner at `(+A, -A)`. Angles are
radians, measured as `atan2(y, x)`.

### `source`: Python or Studio

`source` is an object with a `kind` string and one more key:

| Who wrote it | JSON | Rust |
| --- | --- | --- |
| The Python kit | `{"kind": "python", "commit": "88a51b9f99b4a8367ab5087da92b1fc79aa595c4"}` | `Source::Python { commit }` |
| The Studio | `{"kind": "studio", "version": "0.1.0"}` | `Source::Studio { version }` |

`commit` is the kit's commit: the last git commit that changed `projects/`, `reelkit/` or `series/` (40 hex digits),
with `-dirty` appended if those folders had uncommitted changes. It is not the repo's HEAD, so it does not move when
only Studio files are committed. `version` is the app's version string. Any other `kind` is an error.

### How Python values become JSON

| Python | JSON | `Value` |
| --- | --- | --- |
| `None` | `null` | `Null` |
| `True`, `False` (also numpy bools) | `true`, `false` | `Bool` |
| `int` (also numpy integers); must fit a signed 64-bit integer | a number with no `.`, `e` or `E`: `7`, `-1` | `Int` |
| `float` (also numpy floats; a `float32` is widened exactly) | a number with a `.` or an exponent: `7.0`, `-0.0`, `1e-05`, `2.4000000000000004` | `Float` |
| `str` | a string | `Str` |
| `tuple`, `list`, numpy array, nested to any depth | a list | `List` |

Rules that keep this exact in both directions:

- **Int or Float is decided by the text of the number.** No `.`, `e` or `E`: an integer. Otherwise a float. A writer
  must write a float whose value is whole with a fraction (`100.0`, never `100`), and an integer without one. The
  kit's lists mix the two (a `hit` event has integer fighter indices and float damage), and the content hash tells
  them apart. A reader must classify each number by its text itself: a fraction part or an exponent means `Float`,
  anything else `Int`. A stock JSON parser is not enough for this: `serde_json`, for example, reads `-0` as a float
  and an integer beyond 64 bits as a float. (Python never writes `-0` as an integer, so the reference files are not
  affected; an integer that does not fit a signed 64-bit integer is an error.)
- **Floats are written with the shortest text that reads back to the same bits** (Python's `repr`; in Rust that is
  the `Debug` formatting `{:?}`, which gives `100.0`, `1e-5`, `1e300`. Rust's `Display` `{}` does not: it writes
  `100.0` as `100`, an integer by the rule above, and `1e300` as 301 digits). A reader must convert the text to the
  nearest `f64` (in `serde_json` this needs the `float_roundtrip` feature). Minus zero is written `-0.0` and stays
  minus zero. The digits are the shortest round-trip digits in both languages, and both switch to the exponent form
  at the same points, but the exponent's spelling may differ between writers: Python writes `1e-05` and `1e+16`,
  Rust's `{:?}` writes `1e-5` and `1e16`. Both spellings are valid for a reader and give the same value and the same
  content hash; a file's bytes are therefore only guaranteed identical between writers of the same language.
- **Every float in JSON is finite.** The exporter refuses NaN and infinities there. (`.npy` data may hold NaN: see
  `K` in section 6.3.)
- A Python `dict` inside a value has no JSON form in format 1 and the exporter refuses it. The only objects are
  `meta.json` itself, `source`, `rules` and `arrays`. One rule holds a dict in the kit, `roll_rows` (PIANO's line,
  `{fighter index: [row, ...]}`): it is written as a list of `[fighter index, rows]` pairs in index order (6.1), so
  no object appears inside a value. A recording without a roller has no `roll_rows` and reads exactly as before.

## 3. Frames

All per-frame arrays have the same first dimension. **`frames()` is that number: the length of `HP`.** Recording
frame `i` belongs to video frame `f0 + i`.

- Frame 0 is the state at `rules.t_fight`, before the first step.
- The engine steps at `rules.DT` (1/120 s) and records after every `round(1 / (fps * DT))` steps (4), so frame `i`
  is the state at `t_fight + i * 4 * DT`.
- When a fighter is knocked out, the state right after that step is recorded as the **last** frame, whenever it
  falls: the last frame of a fight with a K.O. is the state at `t_ko`, which may sit between two regular frame
  times. A fight without a K.O. simply ends with its last regular frame, within one frame of `rules.t_max`.

`ep392.bfrec` has 1,394 frames (`f0` = 108, `t_ko` = 50.021568627450975).

## 4. `<KEY>.npy`: rectangular arrays

The NumPy `.npy` format, version 1.0, restricted to two element types:

| Bytes | Content |
| --- | --- |
| 0 to 5 | the magic `93 4E 55 4D 50 59` (`\x93NUMPY`) |
| 6, 7 | version `01 00` |
| 8, 9 | `HEADER_LEN`, unsigned 16-bit little-endian |
| 10 to 10 + `HEADER_LEN` - 1 | the header: ASCII text, a Python dict literal, padded with spaces and ending with one `\n` |
| the rest | the data: `product(shape)` elements of 8 bytes each, little-endian, in C order (the last index runs fastest) |

The header Python writes for `P` in `ep392.bfrec` is exactly

```
{'descr': '<f8', 'fortran_order': False, 'shape': (1394, 2, 2), }
```

followed by spaces and a newline up to `HEADER_LEN` = 118, so the data starts at byte 128. A reader uses
`HEADER_LEN` from the file and does not assume 118. The three keys:

- `descr`: `'<f8'` (IEEE 754 double, `Array::F64`) or `'<i8'` (signed 64-bit integer, `Array::I64`). Anything else
  is an error.
- `fortran_order`: `False`. `True` is an error.
- `shape`: a tuple of non-negative integers. A one-dimensional shape is written `(1394,)`. The first number is the
  frame count.

A writer pads the header with spaces so that `10 + HEADER_LEN` is a multiple of 64 and ends it with `\n` (NumPy then
reads the file). To match NumPy's bytes exactly (not needed for a valid file): NumPy first puts
`21 - len(str(shape[0]))` spare spaces after the dict, then pads with 1 to 64 spaces (64 when the length is already
aligned, never 0), then the `\n`. A reader should find the three keys without depending on their order, spacing or
the padding. The data length must be exactly `8 * product(shape)` bytes.

A key is `<i8` when every value the kit recorded for it is an integer (`FR`, `NV`, `LV`), otherwise `<f8`.

## 5. `<KEY>.json`: ragged arrays

A JSON list with one entry per frame (`frames()` entries). What a frame's entry holds depends on the key (section
6.3): typically a list of things that exist in that frame, each a short list of numbers and strings. The value rules
of section 2 apply. Python writes one frame per line, in ASCII with any non-ASCII character as a `\u` escape; a
reader must not depend on either and must accept any valid UTF-8 JSON. In Rust the whole list is
`Array::Nested(Value::List(...))`.

`WAVES`, `SLAMS` and `COB` are always JSON, in every recording, even when every frame happens to hold the same
number of things. In general: a key that the kit's `story.REC_KEYS` lists is `.npy` whenever its values form a
rectangular array of integers or floats, and every other key is JSON.

## 6. What the fields mean (FLUTE and TROMBONE)

### 6.1 `rules`

Every attribute of the kit's `Rules` object for the fight: the engine's constants and the episode's settings. Rule
overrides of a scenario fight appear here with the value used.

| Key | Type | Reference value | Meaning |
| --- | --- | --- | --- |
| `A` | float | 3.0 | half-size of the ring inside the ropes |
| `R` | float | 0.6 | ball radius (both the same) |
| `V` | float | 2.6 | the shared ball speed, ring units per second |
| `DT` | float | 0.008333333333333333 | the step, 1/120 s |
| `FPS` | int | 30 | recorded frames per second (the same number as `fps`) |
| `ROPE_GIVE` | float | 5.0 | degrees of seeded jitter on a rope bounce |
| `HP` | float | 100.0 | starting hit points |
| `LOW_HP` | float | 25.0 | below this a fighter's `low` ability fires, once |
| `LV_DMG` | list of 2 floats | [14.0, 36.0] | damage dealt to reach level 2 and level 3 |
| `KNOCK_T` | float | 0.38 | seconds a knockback speed boost takes to decay |
| `LAUNCH_SPREAD` | float | 38.0 | degrees either side of the corner-to-centre line at the start, seeded |
| `KEY_R` | float | 0.11 | projectile hit radius |
| `bpm` | float | 133.7704918032787 | the tempo (the episode's 132 fitted to a whole number of bars in 61 s) |
| `beat` | float | 0.44852941176470584 | `60 / bpm`, seconds |
| `t_fight` | float | 3.5882352941176467 | when the fight starts (the bar-2 downbeat) |
| `t_max` | float | 58.0 | the fight stops here if nobody is knocked out |
| `dmg_scale` | float | 1.0 | shared damage scale |
| `dmg_side` | list of 2 floats | [2.0, 0.4] | per-fighter damage handicap (left, right) |
| `swat_cobras` | bool | true | the opt-in rule: a live lance knocks a risen cobra flat |
| `swat_shells` | bool | (absent) | the opt-in rule (#48, EUPHONIUM against TROMBONE): a live lance swats a falling shell |
| `swat_shell_h`, `swat_shell_r` | float | (absent: 0.85, 0.2) | how low a shell must be, and how near the lance in plan, to be swatted |
| `swallow_rollers` | bool | (absent) | the opt-in rule (#56, BASS DRUM against TUBA): a swallowing bell can gulp a rolling drum |
| `roll_rows` | list of `[int, list of lists of ints]` | (absent) | a roll fighter's rows (PIANO, #45; `kit/story.py` 33 to 35): Python's dict `{1: [(64, 52, 60), (), (67,)]}` is `[[1, [[64, 52, 60], [], [67]]]]`, one pair per roller in fighter order, each row its MIDI notes (empty for a rest) |
| `roll_start8` | int | (absent: 0) | the 8th of the video the roll's first row belongs to, `8 * LOOP_START` |

`dmg_side`, `swat_cobras` and the rows below it are attributes only an episode adds, so they can be absent, as in
Python's `vars(rules)`; absent is the default the engine reads (`getattr(R, name, default)`). A recording the Studio makes from a fight file has `dmg_side` unless the file's `handicap` is 1.0 for
each fighter (a file with no `handicap` line; the kit passes no `dmg_side` for an episode without `DMG_SIDE`), and
`swat_cobras` exactly when the file's `pair_rules` set it, with the value set (`false` too: Python's `setattr` lists
an attribute set to false). The fight is the same either way; only the content hash shows it.

One known difference from the kit: an explicit `handicap = [1.0, 1.0]` is the same fight file as no `handicap` line,
so the Studio records no `dmg_side` for it, where Python with `DMG_SIDE = (1.0, 1.0)` records `dmg_side` `[1.0,
1.0]`. It cannot change a fight: the engine's only reader of `dmg_side` multiplies a hit's damage by it. Such a pair of
recordings differs in its content hash only.
`studio/reference/fights/rules_map/` (export `rules_map`) holds a Python recording of each case, and
`crates/bf-sim/tests/rules_map.rs` checks that the Studio's map equals it.

### 6.2 Events

`[t, kind, who, ...]`. `t` is the step's time rounded to 5 decimals (`round(t, 5)`), so several events can share a
time; they are in the order the engine made them. Positions below: element 0 is `t`, 1 is `kind`, 2 is `who`.
`who` is a fighter index (0 or 1), **except in `clash`, where it is -1**.

These twelve kinds are all that FLUTE against TROMBONE produces (checked over seeds 1 to 300, and by reading
`fight.py`: neither fighter has a shield, a projectile or a stun, so `block`, `parry`, `cleared` and `stun` cannot
occur).

| Kind | `who` (2) | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `hit` | attacker | victim (int) | damage taken (float), after `dmg_scale` and `dmg_side` | contact x (float) | contact y (float) | what hit (string, below) | sub (int, below) |
| `thrust` | the lance's owner | the lance's full reach beyond the ball's surface for this thrust (float) | | | | | |
| `basket` | the charmer | basket id (int, counts up from 0 per fighter) | x where it was dropped (float) | y (float) | id of the basket taken back to make room, or -1 (int) | | |
| `strike` | the charmer | basket id (int) | aim angle (float) | length of the lunge (float) | seconds from now until the bite lands (float) | | |
| `swat` | the lance's owner | the cobra's owner (int) | basket id (int) | x of the touch (float) | y (float) | | |
| `ability` | the user | the ability's name (string) | its kind (string) | 1 if this is the first use of that ability, else 0 (int) | | | |
| `level` | the fighter | the level reached, 2 or 3 (int) | | | | | |
| `slow` | the fighter that is slowed | the wave's owner (int) | seconds the slow lasts (float) | | | | |
| `slam` | the slam's owner | centre x (float) | centre y (float) | 1 if the enemy was inside when it landed, else 0 (int) | | | |
| `rope` | the ball that bounced | axis: 0 the left or right rope, 1 the top or bottom rope (int) | which side: +1 or -1 (int) | ball x after the bounce (float) | ball y (float) | its speed then (float) | |
| `clash` | -1 | x of the midpoint between the two balls (float) | y (float) | | | | |
| `ko` | the loser | the winner (int) | | | | | |

`hit`, elements 7 and 8:

| What hit (7) | Cause | Sub (8) |
| --- | --- | --- |
| `blade` | the TROMBONE's lance | how many times the lance had grown before this hit, plus 1 (its `FR` value) |
| `fang` | a FLUTE cobra's bite | the basket id |
| `shockwave` | SAD TROMBONE | 0 |
| `wave` | PICCOLO | 0 |
| `slam` | PEDAL TONE landing | 0; x and y are the victim's position |

`ability`, elements 3 and 4, for these two fighters:

| Fighter | Name | Kind | When |
| --- | --- | --- | --- |
| FLUTE | `FLUTTER TONGUE` | `haste` | from level 2, every 4 bars |
| FLUTE | `SNAKE DANCE` | `dance` | from level 3, every 5 bars |
| FLUTE | `PICCOLO` | `wave` | once, below `LOW_HP` |
| TROMBONE | `SAD TROMBONE` | `shockwave` | from level 2, every 4 bars |
| TROMBONE | `POWER SLIDE` | `haste` | from level 3, every 6 bars |
| TROMBONE | `PEDAL TONE` | `slam` | once, below `LOW_HP` |

Two more kinds, which these two weapons can make but this pairing never does. `block` is in the bag fights
(`studio/reference/fights/striker_vs_bag/` and `lance_vs_bag/`, where a bag has a shield); no reference recording
holds a `parry`, because no ported weapon fires a projectile.

| Kind | `who` (2) | 3 | 4 | 5 | 6 |
| --- | --- | --- | --- | --- | --- |
| `block` | the shielded fighter | the attacker (int) | contact x (float) | contact y (float) | what was blocked (string): `blade` or `fang`, as element 7 of a `hit` |
| `parry` | the projectile's owner | the fighter whose lance killed it (int) | the projectile's x (float) | its y (float) | the projectile's id (int) |

The story reads events like this: captions come from `ability` events whose element 5 is 1; hit points over time
are 100 minus the running sum of element 4 of the `hit` events whose element 3 is that fighter.

### 6.3 Per-frame arrays

"Per fighter" means a last dimension of 2 in the order of `names`. The reference fights have these 19 keys.

| Key | Stored as | Shape | Meaning |
| --- | --- | --- | --- |
| `P` | `<f8` | (frames, 2, 2) | ball centre: `P[i][k]` is fighter `k`'s `[x, y]` |
| `V` | `<f8` | (frames, 2, 2) | velocity `[vx, vy]`, ring units per second |
| `TH` | `<f8` | (frames, 2) | weapon angle, 0 to 2 pi. TROMBONE: where the lance points. FLUTE: fixed at its seeded start value. |
| `BL` | `<f8` | (frames, 2) | weapon reach beyond the ball's surface; it grows with landed hits. FLUTE: the cobras' reach (1.4 up to 2.4). TROMBONE: the lance's full length (0.95 up to 2.47, `2.470000000000001`: it grows by 0.08 per landed thrust only while that stays within `max_len` 2.5). |
| `FR` | `<i8` | (frames, 2) | growth steps so far. TROMBONE starts at 1 and adds 1 each time a landed hit lengthens the lance; FLUTE stays 0. |
| `NV` | `<i8` | (frames, 2) | projectiles per volley (0 for both) |
| `HP` | `<f8` | (frames, 2) | hit points, never below 0 |
| `LV` | `<i8` | (frames, 2) | level, 1 to 3 |
| `SH` | `<f8` | (frames, 2) | seconds of shield left (always 0 for these two) |
| `HA` | `<f8` | (frames, 2) | seconds of haste left |
| `SPIN` | `<f8` | (frames, 2) | spin direction, +1.0 for the left fighter and -1.0 for the right |
| `OM` | `<f8` | (frames, 2) | blade spin rate, radians per second (0 for these two) |
| `EXT` | `<f8` | (frames, 2) | how far the lance sticks out right now, beyond the surface. TROMBONE: 0.3 at rest, up to `BL` in a thrust. FLUTE: 0. |
| `SLW` | `<f8` | (frames, 2) | seconds left of a slow on this fighter |
| `CRV` | `<f8` | (frames, 2) | turn rate of curving projectiles, degrees per second (0 for both) |
| `K` | `<f8` | (frames, 72, 7) | projectiles in flight, up to 72 a frame: `[x, y, heading, owner, id, index in its volley, volley id]`, and every unused slot is NaN. These two fighters fire none, so **`K` is all NaN** here (the bit pattern `7FF8000000000000`). |
| `WAVES` | JSON | frames x (waves in flight) | each wave: `[owner (int), centre x, centre y, radius now, final radius, kind (string: "shockwave" or "wave"), time it started]` |
| `SLAMS` | JSON | frames x (slams pending or just landed) | each slam: `[owner (int), centre x, centre y, time it was called, time it lands, radius]`; it stays listed until 0.6 s after it lands |
| `COB` | JSON | frames x 2 x (that fighter's baskets) | each basket: `[id (int), basket x, basket y, cobra x, cobra y, time it was dropped]`. The cobra sits on its basket except during SNAKE DANCE, when it leaves to chase the enemy and then returns. Only a fight with a `striker` weapon has this key; the TROMBONE's list is always empty. |

Because `K` holds NaN, two recordings must be compared by the bits of their floats, not with `==` on `f64`
(`NaN != NaN`). The same goes for telling `0.0` from `-0.0`.

The first 15 keys and `WAVES` and `SLAMS` are in every recording. The kit adds `EXT`, `SLW` and `CRV` (always the
three together) only when a fighter thrusts, slows or curves its shots, as the TROMBONE does, and `COB` only when a
fighter is a `striker`, as the FLUTE is.

Other fighters add other keys (the kit's `story.REC_KEYS` and `fight.py` list them); a reader keeps every key it
finds and needs no list of known names.

### 6.4 The kit's other kinds (one section per kind, each filled in by its port)

Every weapon and ability kind of the kit's other fighters: what `fight.py` records for it (its keys, the lines of `snap`
that write them) and every event and hit it makes, with the line that makes it, transcribed from the kit. A fighter's
port fills in its kinds' sections (the meaning of every field) and edits only the text between its kinds' markers. A
reader keeps every key it finds, whatever this section says (6.3). The `shield` (TUBA, HORN, ORGAN) is ported: its
`block` is the `block` of 6.2.

<!-- kind:roll -->
#### `roll` (weapon kind of PIANO)

`fight.py` 5164 to 5283, 370 to 379, 3235 to 3278; ported (milestone 3, Task 7; `kinds/roll.rs`). A paper roll lies
along the side of the ring through the fighter's own corner; on every 8th the row under its read bar fires one shot
per hole, from the hole's pitch lane, across the ring and turned toward the enemy. The rows are the fighter's line
(the rule `roll_rows`, 6.1). The roll's module also flies and lands every projectile of the fight (its rows', the
`runoff`'s and an `arc`'s).

**Record keys** (in `rec` when a fighter's weapon is a roll; `run()` 5887, `snap` 5621 to 5624).

- `ROLL`: float, `(frames, 2, 3)`, per fighter `[n_vol, sustain, reload]`: how many holes a row fires (a float: a
  landed row adds `grow`; a row fires `int(n_vol)`, at least 1), the seconds of SUSTAIN left, the seconds left until
  the new roll is threaded (both 0.0 when none). A fighter without a roll has `[n_vol, 0.0, 0.0]`.
- `HOLD`: ragged (`HOLD.json`), per frame the list of SUSTAIN's shots that have not flown on yet, in id order, each
  `[id, t_stop, t_release]`: the projectile's id (a whole number), when it stops in the air and when it turns to the
  enemy and flies on (seconds on the video clock).
- `K` (6.3; every recording has it): a roll's fight is the first to fill it. Each of the first `min(72, shots)` rows
  is `[x, y, heading, owner, id, index, volley]` of one projectile, in the order they were fired: the heading is
  `atan2(vy, vx)` (0.0 for a hanging shot), the index is the shot's place in its volley (a row's or a runoff's shot:
  its MIDI pitch; an arc's: its step along the fan), the volley is the row, runoff or arc it belongs to. Unused rows
  are NaN.

**Events.**

- `roll`: `[t, "roll", fighter, volley, abs8, sustained, [pitch, ...]]` (5243): a row fired: the volley's number,
  the absolute 8th of the video, 1 while the fighter's SUSTAIN holds else 0, and the pitches fired, in the row's
  order (a list of whole numbers; Python's tuple).
- `wall`: `[t, "wall", owner, -1, x, y, id]` (3243, `kill_keys`): a projectile left the ring: its owner, -1 (nobody
  killed it), where it was and its id. (`parry`, 6.2, is the same event with the lance's owner in place of -1.)
- a `hit` of kind `key` (3256): `[t, "hit", attacker, victim, damage, x, y, "key", index]`: a projectile landed, at
  its own position; the sub-number is its index (above). The first landed shot of a roll's volley adds `grow` to
  `n_vol` (up to `n_max`), and every landed shot nudges the victim's heading along the shot's flight.

<!-- /kind:roll -->

<!-- kind:swallow -->
#### `swallow` (weapon kind of TUBA)

`fight.py` 1045 to 1130. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 1062: `self.ev('oom', k, B.sw['t_in'], B.sw['t_pah'], float(B.blade_len))`
- 1082: `self.damage(k, 1 - k, w['dmg_rope'], 'cannon', e[5], e[6])`
- 1104: `self.ev('block', 1 - k, k, hx, hy, 'swallow')`
- 1107: `self.damage(k, 1 - k, w['dmg'], 'gulp', hx, hy)`
- 1113: `self.ev('gulp', k, hx, hy, float(B.blade_len))`
- 1117: `self.ev('pah', k, 0, float(B.th))`
- 1127: `self.ev('pah', k, 1, a)`
- 1128: `self.damage(k, 1 - k, w['dmg_spit'], 'spit', float(O.p[0]), float(O.p[1]))`

<!-- /kind:swallow -->

<!-- kind:echo -->
#### `echo` (weapon kind of HORN)

`fight.py` 2196 to 2281. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `ECHO_G` (and `ECHO_N`, `ECHO_R`): added to `rec` at `run()` 5792, written by `snap` 5539 to 5549:

  ```python
  if 'ECHO_G' in rec:                                 # #34 v5, only recorded when a fighter uses the 'echo' kind
      G = np.full((2, self.EMAX, 3), np.nan)
      for j, Bj in enumerate((B0, B1)):
          if hasattr(Bj, 'echo_hist'):
              for i in range(self.EMAX):              # every slot (the renderer fades the inactive ones)
                  G[j, i, :2] = self.echo_pos(Bj, (i + 1) * Bj.echo_sp)
                  G[j, i, 2] = (i + 1) * Bj.echo_sp
      rec['ECHO_G'].append(G)
      rec['ECHO_N'].append([getattr(B0, 'n_echo', 0.0), getattr(B1, 'n_echo', 0.0)])
      rec['ECHO_R'].append([(E['owner'], float(E['c'][0]), float(E['c'][1]), E['r'], E['rmax'], E['j'], E['t0'],
                             E['call'], int(E['hit'])) for E in self.echoes])
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2234: `self.ev('echo', k, j, call, float(c[0]), float(c[1]), float(rmax))`
- 2251: `self.ev('echo_call', k, B.echo_calls, n, sp16)`
- 2273: `self.damage(k, 1 - k, E['dmg'], 'echo', hx, hy, E['j'])`
- 2279: `self.ev('echo_grow', k, int(B.n_echo + 1e-9))`

<!-- /kind:echo -->

<!-- kind:line -->
#### `line` (weapon kind of ORGAN)

`fight.py` 1323 to 1362 (`hazard_beat`, `_to_ropes`, `fire_line`) and 1393 to 1416 (the lines' half of
`hazards_step`). Ported: `bf-sim/src/kinds/line.rs`, settings in `bf-core/src/roster/kinds/line.rs`. `lay_string` and
the strings' half of `hazards_step` (1364 to 1391, 1417 to 1434) belong to the older `trap` kind, which `line` never
reaches; they are not ported.

**Record keys.** None of its own. The rows live only in the events: a look rebuilds them from `line`. The row's length
for the next row is the ball's `BL` (`blade_len`), which starts at the weapon's `base` and grows by `grow` per landed
row up to `max_len`.

**Events.**

| Kind | `who` (2) | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `line` | the row's owner | hazard id (int, counts up from 0 over the fight) | x where the row starts (float): the ball's surface plus `gap` toward the enemy | y (float) | x of its direction, a unit vector at the enemy as it stood (float) | y of its direction (float) | its length (float): `BL`, or as far as the ropes | `speed`: units a second the eruption front runs out (float) | `linger`: seconds a pipe stays up (float) | `width` (float) |

A row is fired on a beat of the fight, every `max(1, int(round(every / mult)))` beats, where `mult` is the haste factor
while a haste lasts (PULL ALL STOPS: every beat) and 1 otherwise. In the kit the length (element 8) is a numpy float
whenever a rope cut the row short and a Python float otherwise; a recording has one float type, and the value is the
same.

`hit`, elements 7 and 8:

| What hit (7) | Cause | Sub (8) |
| --- | --- | --- |
| `pipe` | a row of pipes: the enemy's centre came within its radius plus half the row's `width` of the part of the row that is up (from `max(0, (el - linger) * speed)` to `min(length, el * speed)` along it, `el` seconds after it was fired). A row hits once. x and y are the point of the enemy's surface nearest the row | the row's hazard id |

A `pipe` on a shielded enemy is a `block` (element 6 `pipe`), and the row is spent all the same. The knock is away from
the row plus 0.4 of its direction, strength `knock`.

<!-- /kind:line -->

<!-- kind:rake -->
#### `rake` (weapon kind of ACOUSTIC)

`fight.py` 528 to 565, 457 to 501, 585 to 700. Not ported yet: its port fills this section in (the meaning of each
field, as 6.2 and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 542: `self.ev('rake', k, int(main), float(B.blade_len), float(B.p[0]), float(B.p[1]), float(aim), float(B.rake_dir), float(S), float(w['stroke']))`
- 472: `self.damage(k, 1 - k, w['dmg'] * (1.0 if B.dash_main else w.get('sub_dmg', 0.5)), 'dash', hx, hy, int(B.dash_main))`
- 490: `self.damage(C['owner'], o, C['dmg'], 'cone', hx, hy)`
- 593: `self.ev('surf_end', Sf['owner'], float(c[0]), float(c[1]), int(tau < Sf['life']))`
- 604: `self.damage(Sf['owner'], o, Sf['dmg'], 'surf', hx, hy, int(Sf['grow']))`
- 624: `self.damage(Rk['owner'], o, Rk['dmg'], 'rake', hx, hy, int(Rk['dir'] > 0))`
- 641: `self.ev('whip', Wp['owner'], j, float(ang), float(Wp['len']))`
- 655: `self.damage(Wp['owner'], 1 - Wp['owner'], Wp['dmg'], 'whip', hx, hy, lw['j'])`
- 668: `self.damage(Sw['owner'], o, Sw['dmg'], 'sweep', hx, hy)`
- 694: `self.damage(Sp['owner'], 1 - Sp['owner'], Sp['dmg'], 'spout', hx, hy, Sp['ticks'])`
- 698: `self.ev('spout_end', Sp['owner'], float(Sp['p'][0]), float(Sp['p'][1]))`

<!-- /kind:rake -->

<!-- kind:combo -->
#### `combo` (weapon kind of TRUMPET)

`fight.py` 898 to 966 (`combo_setup`, `combo_beat`, `combo_step`); the engine's `bf-sim/src/kinds/combo.rs`. Every
`every` beats (haste divides it) the TRUMPET queues `punches` pistons, `gap` beats apart; each aims where the enemy
is when it starts, shoots out to the reach, holds and pulls back, and hits once.

**Record keys.** None of its own. What a recording of a combo shows of it, in the shared keys:

- `TH`: the valves' aim, 0 to 2 pi. It starts facing the enemy and turns toward it at `aim_rate` at most, every step.
- `BL`: the pistons' reach beyond the ball's surface: `base` (0.6), plus `grow` (0.05) per landed punch, up to
  `max_len` (1.7).
- `FR` stays 0, `EXT` is absent (the pistons' lengths are not recorded: a look works them out from the `punch`
  events).

**Events and hits** (`e[0]` is the time, `e[1]` the kind, `e[2]` the TRUMPET's side):

- `combo` (916): a combo starts on its beat. `e[3]` its number, counting from 1 (int); `e[4]` its punches (int, 3);
  `e[5]` the seconds between two punches (`gap` x the beat / the haste factor).
- `punch` (936): a piston starts. `e[3]` its place in its combo, from 0 (int); `e[4]` its aim in radians (-pi to pi:
  where the enemy was at that step); `e[5]` the reach then; `e[6]` its combo's number (int).
- `hit` with `e[7] = "piston"` (961): a punch lands. As every hit: `e[3]` the victim, `e[4]` the damage (`dmg`, or
  `dmg3` for the last punch of a combo, x the scales), `e[5]`, `e[6]` where on the enemy's surface; `e[8]` the
  punch's place in its combo (int).
- `block` with `e[6] = "piston"` (957): a punch met a shield. `e[2]` the shielded fighter, `e[3]` the TRUMPET,
  `e[4]`, `e[5]` where. The piston pulls straight back.

<!-- /kind:combo -->

<!-- kind:jump -->
#### `jump` (weapon kind of CLARINET)

`fight.py` 2338 to 2424. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2390: `self.damage(k, 1 - k, dmg, 'cut', hx, hy, kind)`
- 2394: `self.ev('jump', k, float(p0[0]), float(p0[1]), float(p1[0]), float(p1[1]), int(hit), kind)`

<!-- /kind:jump -->

<!-- kind:tune -->
#### `tune` (weapon kind of OBOE)

`fight.py` 2425 to 2525, ported (milestone 3, Task 11; `kinds/tune.rs`): TUNING A. Every `every` beats the oboe
sounds its A as a narrow beam: the aim locks `lock16` 16ths before the shot, the beam fires `off16` 16ths after the
grid's line, `len` long and `width` wide, and stays live for `hold` seconds, hitting once. A landed beam puts the enemy
OUT OF TUNE for `dur_beats` beats at the oboe's depth (`depth0`, plus `grow` per landed beam, up to `depth_max`): the
detuned fighter's own hits land `weak` x depth weaker (`damage` 344 to 346), and every note of the oboe's `sting16`
grid stings it. Between shots `TH` (the oboe's aim) turns to the enemy at `track` radians a second; a locked aim holds
`TH` until the shot. A wave with the option `detune` (ILL WIND) puts the enemy out of tune when it lands, through the
same `_detune` (`effects_step` 3069 to 3071).

**Record keys** (only in a fight where a fighter has the `tune` weapon; both keys together):

| Key | Stored as | Shape | Meaning |
| --- | --- | --- | --- |
| `DET` | `<f8` | (frames, 2, 2) | per fighter `[seconds left out of tune, depth]`. The seconds run down to 0 each step; the depth stays at its last value after they run out, so "out of tune" is seconds above 0. |
| `TUNE` | `<f8` | (frames, 2, 2) | per fighter `[1.0 while it has a live beam, 1.0 while it holds ENDLESS NOTE's beam]`, else 0.0. |

```python
if 'DET' in rec:                                    # #37, only recorded when a fighter uses the 'tune' kind
    rec['DET'].append([[getattr(B0, 'detune_t', 0.0), getattr(B0, 'detune_d', 0.0)],
                       [getattr(B1, 'detune_t', 0.0), getattr(B1, 'detune_d', 0.0)]])
    rec['TUNE'].append([[float(bool(getattr(Bj, 'tbeams', None))), float(getattr(Bj, 'endless', None) is not None)]
                        for Bj in (B0, B1)])
```

**Events** (positions as in 6.2):

| Kind | `who` (2) | 3 | 4 | 5 | 6 | 7 |
| --- | --- | --- | --- | --- | --- | --- |
| `tune_lock` | the oboe | the locked aim, 0 to 2 pi (float) | | | | |
| `tune` | the oboe | x where the beam leaves the ball (float) | y (float) | its aim (float) | its length (float) | its width (float) |
| `detune` | the fighter put out of tune | who did it (int) | the depth (float) | the seconds it lasts (float) | | |
| `endless_end` | the oboe (ENDLESS NOTE ran out; nothing more) | | | | | |

**Hits** (`hit`, elements 7 and 8):

| What hit (7) | Cause | Sub (8) |
| --- | --- | --- |
| `tune` | a beam of TUNING A; x and y are the near side of the victim's ball | 0 |
| `sting` | a note of the oboe's `sting16` grid while the enemy is out of tune: `sting` x the depth; x and y are the victim's position | 0 |
| `endless` | ENDLESS NOTE's held beam, on an 8th; x and y are the victim's side toward the oboe's aim | 0 |

`fight.py` lines: 2450 `detune`, 2460 `endless_end`, 2471 `endless`, 2481 `tune_lock`, 2491 `tune`, 2494 `sting`,
2519 `tune`.

<!-- /kind:tune -->

<!-- kind:anchor -->
#### `anchor` (weapon kind of BASS CLARINET)

`fight.py` 2568 to 2658. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `ANC`: added to `rec` at `run()` 5826, written by `snap` 5569 to 5576:

  ```python
  if 'ANC' in rec:                                    # #38, only recorded when a fighter uses the 'anchor' kind
      Ac = np.full((2, 6), np.nan)
      for j, Bj in enumerate((B0, B1)):
          An = getattr(Bj, 'anc', None)
          if An is not None:
              Ac[j] = (1.0, An['c'][0], An['c'][1], An['th'], float(np.hypot(*(Bj.p - An['c']))),
                       float(getattr(Bj, 'twin_until', -1.0) > self.t))
      rec['ANC'].append(Ac)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2600: `self.ev('anchor', k, float(c[0]), float(c[1]), float(orbit), B.anc['t_end'])`
- 2609: `self.ev('pull', k, float(B.p[0]), float(B.p[1]), float(B.v[0]), float(B.v[1]))`
- 2649: `self.damage(k, 1 - k, dmg, 'anchor', hx, hy, ghost)`

<!-- /kind:anchor -->

<!-- kind:mortar -->
#### `mortar` (weapon kind of EUPHONIUM)

`fight.py` 2659 to 2766, 1243 to 1271. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `SHELLS`: added to `rec` at `run()` 5817, written by `snap` 5564 to 5568:

  ```python
  if 'SHELLS' in rec:                                 # #38, only recorded when a fighter uses 'mortar' / 'barrage'
      rec['SHELLS'].append([(Sh['owner'], float(Sh['p0'][0]), float(Sh['p0'][1]), float(Sh['p1'][0]),
                             float(Sh['p1'][1]), Sh['t0'], Sh['t1'], Sh['salvo'], Sh['j'], Sh['landed'])
                            + ((Sh['ts'],) if 'ts' in Sh else ())       # #48: a swatted shell's swat time
                            for Sh in self.shells])
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2710: `self.ev('mortar', k, n, salvo, float(tgt[0]), float(tgt[1]))`
- 2754: `self.damage(Sh['owner'], o, Sh['dmg'], 'shell', float(O.p[0]), float(O.p[1]), Sh['j'])`
- 2758: `self.ev('shell', Sh['owner'], float(Sh['p1'][0]), float(Sh['p1'][1]), int(landed), Sh['salvo'], Sh['j'], Sh['kind'])`
- 1270: `self.ev('swat_shell', k, Sh['owner'], Sh['salvo'], Sh['j'], float(p[0]), float(p[1]))`

<!-- /kind:mortar -->

<!-- kind:tiptoe -->
#### `tiptoe` (weapon kind of BASSOON)

`fight.py` 3280 to 3429. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `HOP`: added to `rec` at `run()` 5835, written by `snap` 5609 to 5615:

  ```python
  if 'HOP' in rec:                                    # #39, only recorded when a fighter uses the 'tiptoe' kind
      Hp = np.full((2, 3), -1.0)
      for j, Bj in enumerate((B0, B1)):
          H = getattr(Bj, 'hop', None)
          if H is not None:
              Hp[j] = ((self.t - H['t0']) / max(1e-9, H['t1'] - H['t0']), H['t0'], H['t1'])
      rec['HOP'].append(Hp)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3329: `self.ev('hop', k, float(p0[0]), float(p0[1]), float(p1[0]), float(p1[1]), self.t + air, kind)`
- 3358: `self.damage(k, 1 - k, dmg * (w['back'] if back else 1.0), 'stomp', hx, hy, int(back))`
- 3363: `self.ev('stomp', k, float(B.p[0]), float(B.p[1]), int(hit), int(back), float(B.stomp_r), H['kind'])`
- 3379: `self.ev('mountain_end', k)`

<!-- /kind:tiptoe -->

<!-- kind:lightning -->
#### `lightning` (weapon kind of ELECTRIC)

`fight.py` 3845 to 3922. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3876: `self.ev('riff', k, B.riff_id, B.riff_n)`
- 3912: `self.ev('bolt', k, int(note), -1 if src is None else int(src), x0, y0, float(x1), float(y1), int(hit), int(B.riff_id), int(fork), int(tap))`
- 3917: `self.damage(k, 1 - k, dmg, 'bolt', x1, y1, int(src) + 1)`

<!-- /kind:lightning -->

<!-- kind:grapple -->
#### `grapple` (weapon kind of CELLO)

`fight.py` 4041 to 4201. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `GRAB`: added to `rec` at `run()` 5869, written by `snap` 5625 to 5633:

  ```python
  if 'GRAB' in rec:                                   # #47, only recorded when a fighter uses the 'grapple' kind
      Gb = np.zeros((2, 6))
      for j, Bj in enumerate((B0, B1)):
          G_ = getattr(Bj, 'gr', None)
          if G_ is not None:
              Gb[j, :4] = (self.GRAB_STATES[G_['state']], G_['a'], G_['t0'], G_['t1'])
          if hasattr(Bj, 'power'):
              Gb[j, 4:] = (Bj.blade_len, Bj.power)
      rec['GRAB'].append(Gb)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4076: `self.damage(k, 1 - k, ab['dmg'], 'vibrato', float(O.p[0]), float(O.p[1]))`
- 4089: `self.ev('rush', k, aim, B.gr['t1'])`
- 4095: `self.ev('grab', k, how, x, y, float(B.blade_len))`
- 4108: `self.ev('release', k, why)`
- 4120: `self.damage(k, 1 - k, w['dmg_rope'], 'ropeslam', e[5], e[6])`
- 4121: `self.ev('ropeslam', k, e[5], e[6])`
- 4151: `self.ev('block', 1 - k, k, hx, hy, 'grab')`
- 4154: `self.damage(k, 1 - k, w['dmg'], 'grab', hx, hy)`
- 4171: `self.ev('grab', k, 1, hx, hy, float(B.blade_len))`
- 4172: `self.ev('hold', k, a0, a1, B.gr['t1'], delta * sd)`
- 4189: `self.damage(k, 1 - k, w['dmg_throw'], 'throw', hx, hy)`
- 4192: `self.ev('throw', k, a, float(B.power), hx, hy)`

<!-- /kind:grapple -->

<!-- kind:walk -->
#### `walk` (weapon kind of CONTRABASS)

`fight.py` 3965 to 4040, 3190 to 3232. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `WALK`: added to `rec` at `run()` 5881, written by `snap` 5634 to 5642:

  ```python
  if 'WALK' in rec:                                   # #47, only recorded when a fighter uses the 'walk' kind
      Wk = np.full((2, 4), -1.0)
      for j, Bj in enumerate((B0, B1)):
          S_ = getattr(Bj, 'wstep', None)
          if S_ is not None:
              Wk[j, :3] = ((self.t - S_['t0']) / max(1e-9, S_['t1'] - S_['t0']), S_['t0'], S_['t1'])
          if getattr(Bj, 'walker', False):
              Wk[j, 3] = Bj.blade_len
      rec['WALK'].append(Wk)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4016: `self.ev('step', k, float(B.p[0]), float(B.p[1]), float(p1[0]), float(p1[1]), self.t + dur, B.wstep['kind'])`
- 4039: `self.ev('step_land', k, float(B.p[0]), float(B.p[1]), float(B.blade_len), S['kind'])`
- 3232: `self.ev('clash', -1, mx, my)`

<!-- /kind:walk -->

<!-- kind:spotlight -->
#### `spotlight` (weapon kind of ALTO SAX)

`fight.py` 4369 to 4512 (`spot_setup`, `_spot_line`, `spot_tick`, `spot_pos`, `spot_step`, `start_encore`). Ported:
`bf-sim/src/kinds/spotlight.rs`, settings in `bf-core/src/roster/kinds/spotlight.rs`.

On its own 16th grid, every `int(round(4 * every))` 16ths (`off16` later; a haste divides it), a light comes on for
`sweep16` 16ths. It glides, eased, along a straight line `span` long through the point where the enemy will be at
mid-sweep (`lead` times its motion), square to the enemy's heading, turned by up to `jitter` degrees (one draw from
the fight's generator per sweep, also when `jitter` is 0) and coming in from the fighter's own side; both ends are
held 0.25 inside the ropes. No sweep starts while ENCORE runs, or while the fighter is stunned, in shadow or smashed.
The light's radius for the next sweep is the ball's `BL` (`blade_len`): `base`, plus `grow` per sweep that burned, up
to `max_r`. The burn's damage starts at `dmg` and grows by `dmg_grow` per sweep that burned, up to `dmg_max`; it is
not recorded.

**Record keys.**

- `SPOT` (only when a fighter has the kind): a ragged list, one entry a frame: the lights that are on at that frame
  (from 1e-9 before their start to 1e-9 after their end), oldest first, then the phone lights of the last 8th. Each
  light is 9 values:

  | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
  | --- | --- | --- | --- | --- | --- | --- | --- | --- |
  | owner (int) | x of its centre now (float) | y (float) | radius (float) | kind (int): 0 a sweep, 1 OCTAVE KEY's second light, 2 ENCORE's chase light, 3 a phone light | when it came on (float) | when it goes out (float); for the chase light ENCORE's end, or the step it ended on | hot (int): 1 if it came on during ALTISSIMO | its id (int, counts up from 0 over the fight; -1 for a phone light) |

  A phone light's row is `(the first fighter with the kind, x, y, 0.0, 3, t, t + beat / 2, 0, -1)`, `t` the 8th it
  came on. The phone lights are replaced on every 8th of an ENCORE and cleared when it ends.

**Events.**

| Kind | `who` (2) | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `spot` | the light's owner | its id (int) | x where it starts (float) | y (float) | x where it ends (float) | y (float) | when it comes on (float, not rounded as element 0 is) | when it goes out (float) | its radius (float) | kind (int): 0 a sweep, 1 OCTAVE KEY's second light |
| `burn` | the owner | the light's id (int) | x of the light's centre (float) | y (float) | | | | | | |
| `spot_end` | the owner | the light's id (int) | 1 if it burned the enemy at least once, else 0 (int) | | | | | | | |
| `phones` | the owner | how many phone lights came on (int, `int(n_phone)`) | | | | | | | | |
| `phone_burn` | the owner | x of the phone light that burned (float) | y (float) | | | | | | | |

A `burn` event is written on every 16th on which the enemy's centre is within a live light's radius plus 0.6 R of
its centre, whether or not the hit landed (a shielded enemy gives `block`, then `burn`). `spot_end` comes on the first
step after the light's end; for the chase light it comes on the step ENCORE ends. A sweep that burned grows `BL`
and the burn at its `spot_end`; OCTAVE KEY's second light and the chase light grow nothing. OCTAVE KEY's second light
is square to the first through the same point and runs there and back in the same time.

`hit`, elements 7 and 8:

| What hit (7) | Cause | Sub (8) |
| --- | --- | --- |
| `burn` | a light on a 16th: the light's damage as it was when the light came on (times `hot` during ALTISSIMO; the chase light's is ENCORE's `dmg`). x and y are the enemy's centre. A landed burn knocks the enemy away from the light's centre (along its own heading when it stands within 0.001 of it), strength `knock` | 0 |
| `phone` | a phone light within ENCORE's `phone_r` of the enemy's centre: `phone_dmg`. x and y are the phone light's spot. No knock; a landed one is followed by `phone_burn` | 0 |

<!-- /kind:spotlight -->

<!-- kind:crowd -->
#### `crowd` (weapon kind of TENOR SAX)

`fight.py` 4513 to 4757. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `CROWD` (and `THROWS`): added to `rec` at `run()` 5899, written by `snap` 5682 to 5704:

  ```python
  if 'CROWD' in rec:                                  # #51, only recorded when a fighter uses the 'crowd' kind
      Cr = np.full((2, 7), -1.0)
      for j, Bj in enumerate((B0, B1)):
          if not hasattr(Bj, 'throw_dmg'):
              continue
          Cr[j, 0] = Bj.n_vol
          if Bj.pose_until >= 0:
              Cr[j, 1] = (self.t - Bj.pose_t0) / max(1e-9, Bj.pose_until - Bj.pose_t0)
          Cr[j, 2] = float(Bj.hush is not None)
          if Bj.strut is not None:
              St = Bj.strut
              Cr[j, 3] = (self.t - St['t0']) / max(1e-9, St['t1'] - St['t0'])
          Cr[j, 4:] = 0.0
          if Bj.dive is not None:
              Dv = Bj.dive
              ph = Dv['phase']
              a_, b_ = ((Dv['t0'], Dv['t_surf']) if ph == 1 else (Dv['t_surf'], Dv['t_launch']) if ph == 2
                        else (Dv['t_launch'], Dv['t_land']))
              Cr[j, 4:] = (ph, a_, b_)
      rec['CROWD'].append(Cr)
      rec['THROWS'].append([(Th['owner'], float(Th['p0'][0]), float(Th['p0'][1]), float(Th['p1'][0]),
                             float(Th['p1'][1]), Th['t0'], Th['t1'], Th['item'], Th['landed'], Th['id'])
                            for Th in self.throws])
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4585: `self.ev('throw', k, tid, float(o[0]), float(o[1]), float(tgt[0]), float(tgt[1]), self.t, self.t + fl, item, P_['vol'])`
- 4596: `self.ev('subtone', k)`
- 4607: `self.ev('pose', k, B.pose_until, float(B.n_vol))`
- 4630: `self.damage(k, 1 - k, Th['dmg'], 'throw', float(Th['p1'][0]), float(Th['p1'][1]))`
- 4641: `self.ev('splat', k, Th['id'], float(Th['p1'][0]), float(Th['p1'][1]), int(landed))`

<!-- /kind:crowd -->

<!-- kind:drill -->
#### `drill` (weapon kind of SOUSAPHONE)

`fight.py` 4758 to 5005; ported in `bf-sim/src/kinds/drill.rs` (`bf-core/src/roster/kinds/drill.rs` for its roster
numbers). The marcher never glides: it is a walker (it stands planted between sets and only slides when knocked).
Every `every` beats on the 16th grid (haste divides the grid) it marches a drill set: `steps` steps of `step16` 16ths
along one straight line toward where the enemy will be at the blast, at most `span` long, stopping a radius short of
the ropes and `stop` short of that spot. While it marches it cannot be knocked and gives no way in a clash; it
tramples the enemy on contact (at most once every `cool` s) and every enemy basket near its rim. `halt16` 16ths after
the last footfall the bell blast fires straight ahead: a cone that hits once. A landed trample or blast lengthens the
blast (`blade_len`, recorded in `BL` and in `DRILL` column 3). The generator is drawn once per set started (the
line's jitter) and once per HORN FLASH (the way it turns).

**Record keys.**

- `DRILL`: `<f8`, shape `(frames, 2, 16)`, in a recording where a fighter's weapon is a `drill` (`run()` 5905,
  `snap` 5643 to 5670). One row per fighter; the row of a fighter that is not a driller is all NaN. A column is NaN
  while what it describes is not live.

  | Column | Meaning |
  | --- | --- |
  | 0 | 0.0 between sets, 1.0 marching, 2.0 halted before the blast |
  | 1 | the set's progress: `(t - set start) / (last footfall - set start)`, past 1 during the halt (a set only) |
  | 2 | the direction of the set's line in radians; while a blast is live, the blast's direction |
  | 3 | the blast's length now (`blade_len`) |
  | 4 | the live blast's progress, 0 at the shot to (just under) 1 at the end of its hold |
  | 5, 6 | the live blast's length and its width at the bell |
  | 7, 8 | HORN FLASH: the lane's angle now, and its length |
  | 9, 10, 11 | COMPANY FRONT: the wall's position along its axis, the axis (0.0 = x, 1.0 = y), the way it marches (1.0 or -1.0) |
  | 12, 13 | the live blast's origin (the bell), x and y |
  | 14 | the footfalls of the set so far (a set only) |
  | 15 | COMPANY FRONT's progress, 0 to 1 |

**Events** (`who` is the marcher; coordinates are ring units; times are on the video clock):

| Kind | Fields after `who` | When |
| --- | --- | --- |
| `drill` | `x0, y0, x1, y1` (from, to), the set's start time, the time of its last footfall, the steps (int), 1 under haste else 0 (int) | a set starts (4856) |
| `drill_step` | the footfall's number from 1 (int), `x, y` where it lands | each footfall, on the grid (4814) |
| `drill_halt` | `x, y` | after the last footfall: the ball stands at the line's end (4819) |
| `drill_off` | none | a set called off: the marcher is stunned, sunk in shadow, grabbed or in pieces (4808); no blast follows |
| `blast` | `x, y` (the bell), the angle, the length, the width at the bell, the width at the far end | the bell blast fires (4866) |
| `blast_hit` | `x, y` on the enemy's surface | the blast touches the enemy (4928), whether the hit landed or a shield blocked it |
| `drill_trample` | `x, y` between the two balls | the march touches the enemy (4914), landed or blocked |
| `trample_basket` | the basket's id (int), `x, y` of its cobra | a basket of the enemy's is trampled by the march (4884) or by COMPANY FRONT's wall (4977); the basket is gone |
| `flash_hit` | `x, y` on the enemy's surface | HORN FLASH's lane touches the enemy (4945) |
| `front_step` | the step's number from 1 (int), the wall's position | each beat of COMPANY FRONT (4958) |
| `front_hit` | `x, y` (the enemy's centre) | the wall meets the enemy (4969) |
| `front_end` | none | the wall has crossed the ring (4983) |

**Hits** (`hit` events, 6.2; the sub-number is 0): `trample` (4910: `trample_dmg`; the enemy is knocked on ahead
along the line and out of it sideways), `blast` (4925: `dmg`), `flash` (4943: HORN FLASH's `dmg`, knocked outward),
`front` (4965: COMPANY FRONT's `dmg`, knocked on ahead of the wall). A shield turns each into a `block`.

<!-- /kind:drill -->

<!-- kind:roll_charge -->
#### `roll_charge` (weapon kind of BASS DRUM)

`fight.py` 5284 to 5431. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `RCHG`: added to `rec` at `run()` 5911, written by `snap` 5713 to 5732:

  ```python
  if 'RCHG' in rec:                                   # #56, only recorded when a fighter uses the 'roll_charge' kind
      Ro = np.full((2, 9), np.nan)
      for j, Bj in enumerate((B0, B1)):
          if not getattr(Bj, 'roller', False):
              continue
          L = Bj.rl
          Ro[j, 0] = 0.0 if L is None else (1.0 if L['phase'] == 'tip' else 2.0)
          if L is not None:
              if L['phase'] == 'tip':
                  Ro[j, 1] = (self.t - L['t_tip']) / max(1e-9, L['t0'] - L['t_tip'])
              else:
                  Ro[j, 1] = (self.t - L['t0']) / max(1e-9, L['t1'] - L['t0'])
              Ro[j, 2] = math.atan2(L['u'][1], L['u'][0])
              Ro[j, 8] = L['kind']
          Ro[j, 3] = Bj.roll_ang
          Ro[j, 4] = Bj.blade_len
          Ro[j, 5] = Bj.roll_v
          Ro[j, 6] = float(Bj.droll is not None)
          Ro[j, 7] = float(Bj.rolling)
      rec['RCHG'].append(Ro)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 5339: `self.ev('roll', k, float(B.p[0]), float(B.p[1]), float(p1[0]), float(p1[1]), self.t, t0, B.rl['t1'], kind)`
- 5346: `self.ev('roll_off', k, off)`
- 5348: `self.ev('roll_end', k, float(B.p[0]), float(B.p[1]))`
- 5354: `self.ev('drum_roll_end', k)`
- 5391: `self.ev('drum_roll_end', k)`
- 5395: `self.ev('drum_roll_end', k)`
- 5423: `self.damage(k, 1 - k, dmg, 'roll' if ab is None else 'drumroll', hx, hy)`
- 5428: `self.ev('roll_boom', k, hx, hy, L['kind'])`

<!-- /kind:roll_charge -->

<!-- kind:shield -->
#### `shield` (ability kind of TUBA, HORN, ORGAN)

`fight.py` 2847 to 2848, ported (milestone 3, Task 3; `abilities/shield.rs`): `trigger` sets `B.shield = ab_dur(ab)`,
which `SH` records (6.3) and the fight loop runs down (6097). While it holds, a hit on the fighter is the `block` of 6.2
(`damage` 335 to 337; the striker's bite and the thrust lance write their own, with `fang` and `blade`).

**Record keys.** None of its own (`SH`). **Events.** None of its own (`ability`, `block`).

<!-- /kind:shield -->

<!-- kind:sustain -->
#### `sustain` (ability kind of PIANO)

`fight.py` 2941 to 2943, ported (milestone 3, Task 7; `abilities/sustain.rs`; the roll reads it in `fire_row` 5236 to
5242 and `hold_step` 5262 to 5282). For the ability's time each shot of a fired row flies for `out16` 16ths (plus
`step16` for each hole before it), stops, hangs for `hang_beats` beats, then turns to the enemy and flies on.

**Record keys.** None of its own: the seconds left are `ROLL[.., 1]` and the hanging shots `HOLD` (the roll's).

**Events.**

- `sustain`: `[t, "sustain", fighter, until]` (2943): the time (seconds on the video clock) the sustain ends.
  A row fired before then has 1 as its `roll` event's `sustained`.

<!-- /kind:sustain -->

<!-- kind:arc -->
#### `arc` (ability kind of PIANO)

`fight.py` 2852 to 2858, 3099 to 3113, ported (milestone 3, Task 7; `abilities/arc.rs`). Over `beats` beats the ball
throws `n` projectiles one after another in a fan `sweep` degrees wide, centred on where the enemy stood when the
ability fired. The whole arc is one volley, which no `roll` or `runoff` event names: a `K` row whose volley has no
such event is an arc's shot, and its index is its step along the fan (0 to `n - 1`).

**Record keys.** None of its own (its shots are in `K`).

**Events and hits.** None of its own (an `ability` event, from `trigger`, as every ability has; its shots' `wall`
events and `key` hits are the projectiles', see `roll`).

<!-- /kind:arc -->

<!-- kind:runoff -->
#### `runoff` (ability kind of PIANO)

`fight.py` 2944 to 2945, 5245 to 5261, ported (milestone 3, Task 7; `abilities/runoff.rs`, `kinds/roll.rs`
`start_runoff`). The roll runs off its spool: `n` shots fire at once from lanes spread over the whole side, straight
across the ring; then no row fires for `reload_beats` beats.

**Record keys.** None of its own: the seconds until the new roll is threaded are `ROLL[.., 2]` (the roll's).

**Events.**

- `runoff`: `[t, "runoff", fighter, volley, n, [pitch, ...]]` (5260): the volley's number, how many shots, and
  each shot's pitch (its lane's note, rounded half to even), from the low end of the roll to the high end (a list of
  whole numbers; Python's tuple).

<!-- /kind:runoff -->

<!-- kind:beam -->
#### `beam` (ability kind of TRUMPET)

`fight.py` 2870 to 2871, 2990 to 3010, 3013 to 3016, 857 to 897 (`ab_beam_charge`, `ab_beam_fire`, the charged beam
in `abilities_step`, the ability beams of `beams_step`); the engine's `bf-sim/src/abilities/beam.rs`. HIGH C: the
fighter charges for `charge_beats` beats with its aim locked on where the enemy is predicted to be, then fires one
blast that is live for `hold` seconds, hits once and clears the enemy's projectiles in its path. It grows no weapon.

**Record keys.** None of its own: neither the charge nor the live blast is recorded per frame (a look reads the
`charge` and `beam` events).

**Events and hits** (`e[2]` is the fighter):

- `ability` (2839): as every ability, with `e[4] = "beam"`.
- `charge` (2999): the charge starts. `e[3]` the locked aim in radians (-pi to pi); `e[4]` the seconds of charge
  (`charge_beats` x the beat); `e[5]` the blast's length; `e[6]` its width.
- `beam` (3009): the blast fires, in the first step at or after the charge's end. `e[3]`, `e[4]` where it starts
  (the ball's centre then, `R + 0.04` out along the aim); `e[5]` the aim; `e[6]` its length; `e[7]` its width.
- `hit` with `e[7] = "beam"` (884): the blast reaches the enemy (the enemy's centre within `R + width / 2` of the
  blast's line). `e[4]` the damage (`dmg` x the scales), `e[5]`, `e[6]` the point of the enemy's surface nearest
  the line, `e[8]` 0. A shielded enemy gives `block` with `e[6] = "beam"` instead; either way it is knocked along
  the blast.
- `cleared` (895): an enemy projectile within `width / 2 + KEY_R` of the blast's line is taken out. `e[2]` the
  projectile's owner, `e[3]` the fighter that fired the blast, `e[4]`, `e[5]` where it was, `e[6]` its id (int).

<!-- /kind:beam -->

<!-- kind:stun -->
#### `stun` (ability kind of CLARINET)

`fight.py` 2889 to 2892, 3072 to 3074. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3074: `self.ev('stun', o, W['owner'], W['stun'])`

<!-- /kind:stun -->

<!-- kind:rhapsody -->
#### `rhapsody` (ability kind of CLARINET)

`fight.py` 2930 to 2932. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits.** None of its own (an `ability` event, from `trigger`, as every ability has).

<!-- /kind:rhapsody -->

<!-- kind:snip -->
#### `snip` (ability kind of OBOE (and BASS CLARINET))

`fight.py` 2526 to 2567, 2882 to 2883, ported (milestone 3, Task 11; `abilities/snip.rs`): DOUBLE REED (and BASS
CLARINET's SLAP TONGUE, and a snip IDENTITY CRISIS copies). Two blades scissor out of the ball at the enemy, turning
to it at `track` radians a second while they open over `open16` 16ths, and snap shut: a hit when the enemy is then
within `reach` beyond the two surfaces and `half` degrees of the aim (`dmg`, `knock`). The unlock waits in
`abilities_step` (3037 to 3039; 3046 to 3049 for a copy) until the enemy is within `ready` beyond the surfaces. The
shut blades stay for 0.25 s.

**Record keys** (only in a fight where a fighter has a `snip` ability of its own; a copied snip alone adds none):

| Key | Stored as | Shape | Meaning |
| --- | --- | --- | --- |
| `SNIP` | `<f8` | (frames, 2, 3) | per fighter `[state, aim, time of the snap]`: state 1.0 while the blades open, 2.0 once shut; the aim in radians, -pi to pi (it follows the enemy while they open); all three NaN for a fighter with no blades out. |

```python
if 'SNIP' in rec:                                   # #37, only recorded when a fighter has a 'snip' ability
    Sp = np.full((2, 3), np.nan)
    for j, Bj in enumerate((B0, B1)):
        Sn = getattr(Bj, 'snip', None)
        if Sn is not None:
            Sp[j] = (Sn['state'], Sn['aim'], Sn['t_snap'])
    rec['SNIP'].append(Sp)
```

**Events** (positions as in 6.2):

| Kind | `who` (2) | 3 | 4 | 5 |
| --- | --- | --- | --- | --- |
| `snip_open` | the snipper | the time the blades will snap (float) | the aim they open at, -pi to pi (float) | |
| `snip` | the snipper | 1 if the snap hit, else 0 (int) | the aim at the snap (float) | the reach (float) |

**Hits** (`hit`, elements 7 and 8): `snip`, the snap landing; x and y are the near side of the victim's ball; sub 0.

`fight.py` lines: 2541 `snip_open`, 2561 the hit, 2563 `snip`.

<!-- /kind:snip -->

<!-- kind:endless -->
#### `endless` (ability kind of OBOE)

`fight.py` 2884 to 2888, ported (milestone 3, Task 11; `kinds::tune::start_endless`): ENDLESS NOTE. The oboe's
beam is held for the ability's `dur_beats` (its live beams and its aim lock are dropped, and the normal shots pause):
`len` long and `width` wide, its aim (`TH`) turning to the enemy at the ability's `track` radians a second. On every
8th the enemy touches it: `dmg` (a `hit` of kind `endless`, see `tune`), and out of tune again at the oboe's depth.
When the time is up the tune's grid emits `endless_end`.

**Record keys.** None of its own (`TUNE`'s second number, see `tune`).

**Events** (positions as in 6.2):

| Kind | `who` (2) | 3 |
| --- | --- | --- |
| `endless` | the oboe | the time the held beam ends (float) |

`fight.py` line: 2888.

<!-- /kind:endless -->

<!-- kind:flash_spin -->
#### `flash_spin` (ability kind of SOUSAPHONE)

`fight.py` 2977 to 2979, 4985 to 4994; ported in `bf-sim/src/kinds/drill.rs` (`start_flash`). HORN FLASH: the whole
horn spins once over `beats` beats, its blast (`len_k` x the blast's length, `width` wide) sweeping all round from
facing away from the enemy; it hits once (`dmg`, `knock`, outward). The way it turns is drawn from the fight's
generator. It does something only on a driller's ball (`hasattr(B, 'march')`).

**Record keys.** None of its own: `DRILL` columns 7 and 8 (the `drill` section).

**Events.**

| Kind | Fields after `who` | When |
| --- | --- | --- |
| `horn_flash` | the angle it starts at, the time it ends, the lane's length | HORN FLASH starts (4993) |

Its hit is `flash`, with a `flash_hit` event (the `drill` section).

<!-- /kind:flash_spin -->

<!-- kind:company_front -->
#### `company_front` (ability kind of SOUSAPHONE)

`fight.py` 2980 to 2982, 4995 to 5005; ported in `bf-sim/src/kinds/drill.rs` (`start_front`). COMPANY FRONT: a wall
across the whole ring marches from the rope behind the marcher to the rope beyond the enemy, along whichever axis
parts the two balls most, one step a beat for `int(beats)` beats; the enemy it meets (within half of `thick` of the
wall, `thick` 0.35 unless given) takes `dmg` and is knocked on ahead (`knock`); it tramples the enemy's baskets as it
goes. It does something only on a driller's ball (`hasattr(B, 'march')`).

**Record keys.** None of its own: `DRILL` columns 9, 10, 11 and 15 (the `drill` section).

**Events.**

| Kind | Fields after `who` | When |
| --- | --- | --- |
| `company_front` | the axis (int: 0 = x, 1 = y), the way (1.0 or -1.0), the start time, the end time | COMPANY FRONT starts (5004) |

Its hit is `front`, with `front_step`, `front_hit`, `trample_basket` and `front_end` events (the `drill` section).

<!-- /kind:company_front -->

<!-- kind:runway -->
#### `runway` (ability kind of TENOR SAX)

`fight.py` 2966 to 2968, 4648 to 4691. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4663: `self.ev('runway', k, float(B.p[0]), float(B.p[1]), float(p1[0]), float(p1[1]), self.t, t1)`
- 4673: `self.ev('strut_end', k)`
- 4683: `self.damage(k, 1 - k, ab['dmg'], 'strut', hx, hy)`
- 4685: `self.ev('strut_bump', k, hx, hy)`

<!-- /kind:runway -->

<!-- kind:subtone -->
#### `subtone` (ability kind of TENOR SAX)

`fight.py` 2969 to 2972. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2972: `self.ev('hush', k, B.hush['t_throw'])`

<!-- /kind:subtone -->

<!-- kind:stagedive -->
#### `stagedive` (ability kind of TENOR SAX)

`fight.py` 2973 to 2976, 4692 to 4757. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4708: `self.ev('dive', k, float(out[0]), float(out[1]), t_surf, t_launch)`
- 4737: `self.ev('dive_launch', k, float(B.p[0]), float(B.p[1]), float(tgt[0]), float(tgt[1]), Dv['t_land'])`
- 4745: `self.damage(k, 1 - k, ab['dmg'], 'dive', float(O.p[0]), float(O.p[1]))`
- 4748: `self.ev('dive_hit', k, float(O.p[0]), float(O.p[1]))`
- 4756: `self.ev('dive_land', k, float(B.p[0]), float(B.p[1]))`

<!-- /kind:stagedive -->

<!-- kind:octave -->
#### `octave` (ability kind of ALTO SAX)

`fight.py` 2958 to 2960. Ported: `bf-sim/src/kinds/spotlight.rs` (`start_octave`), settings in
`bf-core/src/roster/abilities/octave.rs`. For `dur` seconds (or `dur_beats` beats) every new sweep brings a second
light (`spot`, kind 1). A sweep already running is not changed. On a fighter without a spotlight it does nothing
but its event.

**Record keys.** None of its own: the second lights are `SPOT` rows of kind 1.

**Events.**

| Kind | `who` (2) | 3 |
| --- | --- | --- |
| `octave` | the fighter | when it ends (float) |

<!-- /kind:octave -->

<!-- kind:altissimo -->
#### `altissimo` (ability kind of ALTO SAX)

`fight.py` 2961 to 2963. Ported: `bf-sim/src/kinds/spotlight.rs` (`start_altissimo`), settings in
`bf-core/src/roster/abilities/altissimo.rs`. For `dur` seconds (or `dur_beats` beats) a new sweep's light is
`narrow` times as wide, burns `hot` times as much and its line is `span_k` times as long. A sweep already running
is not changed.

**Record keys.** None of its own: a light that came on during it has `hot` 1 in its `SPOT` rows.

**Events.**

| Kind | `who` (2) | 3 |
| --- | --- | --- |
| `altissimo` | the fighter | when it ends (float) |

<!-- /kind:altissimo -->

<!-- kind:encore -->
#### `encore` (ability kind of ALTO SAX)

`fight.py` 2964 to 2965 and `start_encore` 4501 to 4512; the chase and the phones in `spot_step` 4474 to 4488 and
`spot_tick` 4419 to 4427. Ported: `bf-sim/src/kinds/spotlight.rs` (`start_encore`), settings in
`bf-core/src/roster/abilities/encore.rs`. The fighter's lights are taken away (no `spot_end` for them) and one chase
light comes on where the fighter stands, `max_r` times `r_k` wide, for `beats` beats: every step it moves `speed`
times the shared speed toward the enemy, and it burns `dmg` on every 16th like any light. On every 8th
`int(n_phone)` phone lights pick spots within `spread` of the enemy (two draws from the fight's generator each).

**Record keys.** None of its own: the chase light is a `SPOT` row of kind 2, the phone lights rows of kind 3.

**Events.**

| Kind | `who` (2) | 3 |
| --- | --- | --- |
| `encore` | the fighter | when it ends (float) |

Its `phones`, `phone_burn`, `burn` and `spot_end` events and its `burn` and `phone` hits are in the `spotlight`
section above.

<!-- /kind:encore -->

<!-- kind:twin -->
#### `twin` (ability kind of BASS CLARINET)

`fight.py` 2897 to 2899. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2899: `self.ev('twin', k, B.twin_until)`

<!-- /kind:twin -->

<!-- kind:lights_out -->
#### `lights_out` (ability kind of BASS CLARINET)

`fight.py` 2767 to 2800, 2895 to 2896. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `LO`: added to `rec` at `run()` 5808, written by `snap` 5562 to 5563:

  ```python
  if 'LO' in rec:                                     # #38, only recorded when a fighter has 'lights_out'
      rec['LO'].append([getattr(B0, 'shadow', 0.0), getattr(B1, 'shadow', 0.0)])
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2774: `self.ev('lights_out', k, B.lo['t_end'])`
- 2790: `self.damage(k, 1 - k, ab['dmg'], 'erupt', float(O.p[0]), float(O.p[1]))`
- 2792: `self.ev('erupt', k, float(B.p[0]), float(B.p[1]), int(hit))`

<!-- /kind:lights_out -->

<!-- kind:slap -->
#### `slap` (ability kind of CONTRABASS)

`fight.py` 2946 to 2947, 4202 to 4219. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4218: `self.ev('slap', k, broke)`

<!-- /kind:slap -->

<!-- kind:stampede -->
#### `stampede` (ability kind of CONTRABASS)

`fight.py` 2953 to 2954, 4302 to 4368. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4321: `self.ev('stampede', k, B.stamp['t1'])`
- 4333: `self.ev('stampede_end', k)`
- 4348: `self.ev('rope', k, ax, int(math.copysign(1, B.p[ax])), float(B.p[0]), float(B.p[1]), float(sp))`
- 4354: `self.damage(k, 1 - k, ab['dmg'], 'trample', hx, hy)`
- 4357: `self.ev('trample', k, hx, hy)`
- 4366: `self.ev('stampede_stomp', k, float(B.p[0]), float(B.p[1]), rmax)`

<!-- /kind:stampede -->

<!-- kind:hook -->
#### `hook` (ability kind of BASSOON)

`fight.py` 2915 to 2916, 3430 to 3476. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `HOOK`: added to `rec` at `run()` 5853, written by `snap` 5602 to 5608:

  ```python
  if 'HOOK' in rec:                                   # #39, only recorded when a fighter has a 'hook' ability
      Hq = np.full((2, 3), np.nan)
      for j, Bj in enumerate((B0, B1)):
          Hk = getattr(Bj, 'hook', None)
          if Hk is not None:
              Hq[j] = (Hk['state'], Hk['aim'], Hk['t_hook'])
      rec['HOOK'].append(Hq)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3446: `self.ev('hook_open', k, B.hook['t_hook'], aim)`
- 3466: `self.damage(k, 1 - k, ab['dmg'], 'hook', hx, hy)`
- 3472: `self.ev('hook', k, int(hit), Hk['aim'], float(ab['reach']))`

<!-- /kind:hook -->

<!-- kind:mountain -->
#### `mountain` (ability kind of BASSOON)

`fight.py` 2917 to 2918, 3423 to 3429. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3428: `self.ev('mountain', k, len(ab['seq16']))`

<!-- /kind:mountain -->

<!-- kind:flick -->
#### `flick` (ability kind of HORN)

`fight.py` 2880 to 2881, 2282 to 2337. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.**

- `FLICK`: added to `rec` at `run()` 5829, written by `snap` 5705 to 5711:

  ```python
  if 'FLICK' in rec:                                  # #34 v5, only recorded when a fighter has a 'flick' ability
      Fl = np.full((2, 6), np.nan)
      for j, Bj in enumerate((B0, B1)):
          F = getattr(Bj, 'flick', None)
          if F is not None:
              Fl[j] = (self.FLICK_STATES[F['state']], F['aim'], F['ext'], F['t0'], F['t_hit'], F['hit'])
      rec['FLICK'].append(Fl)
  ```

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2295: `self.ev('flick_wind', k, B.flick['t_hit'], aim)`
- 2304: `self.damage(k, 1 - k, Fr['dmg'], 'flick_rope', e[5], e[6])`
- 2327: `self.damage(k, 1 - k, ab['dmg'], 'flick', hx, hy)`
- 2330: `self.ev('flick', k, 1, F['aim'], float(F['ext']), int(landed))`
- 2334: `self.ev('flick', k, 0, F['aim'], float(reach), 0)`

<!-- /kind:flick -->

<!-- kind:whip -->
#### `whip` (ability kind of ACOUSTIC)

`fight.py` 2872 to 2873, 566 to 584. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 577: `self.ev('sweep', k, float(B.p[0]), float(B.p[1]), float(aim), float(ab['reach']), float(ab['beats'] * self.R.beat))`
- 583: `self.ev('spout', k, float(p[0]), float(p[1]), float(aim))`

<!-- /kind:whip -->

<!-- kind:sweep -->
#### `sweep` (ability kind of ACOUSTIC)

`fight.py` 2872 to 2873, 566 to 584. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 577: `self.ev('sweep', k, float(B.p[0]), float(B.p[1]), float(aim), float(ab['reach']), float(ab['beats'] * self.R.beat))`
- 583: `self.ev('spout', k, float(p[0]), float(p[1]), float(aim))`

<!-- /kind:sweep -->

<!-- kind:fork -->
#### `fork` (ability kind of ELECTRIC)

`fight.py` 2933 to 2935. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2935: `self.ev('fork', k, B.fork_until)`

<!-- /kind:fork -->

<!-- kind:tapping -->
#### `tapping` (ability kind of ELECTRIC)

`fight.py` 2936 to 2938. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2938: `self.ev('tapping', k, B.tap_until)`

<!-- /kind:tapping -->

<!-- kind:smash -->
#### `smash` (ability kind of ELECTRIC)

`fight.py` 2939 to 2940, 3923 to 3964, 6104 to 6107. Not ported yet: its port fills this section in (the meaning of each
field, as 6.2 and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 3934: `self.ev('smash', k, float(B.p[0]), float(B.p[1]), float(ab['radius']), float(B.smashed), int(ab['n']))`
- 3946: `self.ev('splinter', k, j, float(a), float(ab['splinter_speed']))`
- 3956: `self.damage(Sp['owner'], o, Sp['dmg'], 'splinter', hx, hy, Sp['j'])`
- 3959: `self.ev('splinter_end', Sp['owner'], Sp['j'], hx, hy, 1)`
- 3962: `self.ev('splinter_end', Sp['owner'], Sp['j'], float(p[0]), float(p[1]), 0)`
- 6107: `self.ev('rebuild', k)`

<!-- /kind:smash -->

<!-- kind:vibrato -->
#### `vibrato` (ability kind of CELLO)

`fight.py` 2955 to 2957. Not ported yet: its port fills this section in (the meaning of each field, as 6.2 and 6.3 do
for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2957: `self.ev('vibrato', k, B.vib_until)`

<!-- /kind:vibrato -->

<!-- kind:prelude -->
#### `prelude` (ability kind of CELLO)

`fight.py` 2948 to 2950, 4220 to 4243. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2950: `self.ev('prelude', k, self.t + ab['beats'] * self.R.beat)`
- 4239: `self.ev('prelude_note', k, j, float(B.p[0]), float(B.p[1]), rmax)`

<!-- /kind:prelude -->

<!-- kind:swan -->
#### `swan` (ability kind of CELLO)

`fight.py` 2951 to 2952, 4244 to 4301. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 4271: `self.ev('swan', k, self.t, tg, td, tl, float(p0[0]), float(p0[1]), float(c[0]), float(c[1]), float(p1[0]), float(p1[1]), float(ab.get('height', 1.5)))`
- 4298: `self.damage(k, 1 - k, ab['dmg'], 'swan', float(O.p[0]), float(O.p[1]))`
- 4300: `self.ev('swan_land', k, float(B.p[0]), float(B.p[1]), hit)`

<!-- /kind:swan -->

<!-- kind:heartbeat -->
#### `heartbeat` (ability kind of BASS DRUM)

`fight.py` 2983 to 2984, 5432 to 5450. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 5444: `self.ev('heartbeat', k, i, float(B.p[0]), float(B.p[1]))`

<!-- /kind:heartbeat -->

<!-- kind:drum_roll -->
#### `drum_roll` (ability kind of BASS DRUM)

`fight.py` 2985 to 2986, 5451 to 5476. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 5470: `self.ev('roll_off', k, 'drum_roll')`
- 5473: `self.ev('drum_roll', k, B.droll['t1'])`

<!-- /kind:drum_roll -->

<!-- kind:cannons -->
#### `cannons` (ability kind of BASS DRUM)

`fight.py` 2987 to 2988, 5477 to 5494. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 5489: `self.ev('cannon', k, C['i'], float(tgt[0]), float(tgt[1]), self.t + lead)`

<!-- /kind:cannons -->

<!-- kind:copy -->
#### `copy` (ability kind of EUPHONIUM)

`fight.py` 2893 to 2894, 2801 to 2822. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2812: `self.ev('copy', k, src['name'], src['kind'])`
- 2817: `self.ev('copy', k, O.cfg['weapon'].get('name', O.w['kind'].upper()), O.w['kind'])`

<!-- /kind:copy -->

<!-- kind:barrage -->
#### `barrage` (ability kind of EUPHONIUM)

`fight.py` 2900 to 2901, 2721 to 2740. Not ported yet: its port fills this section in (the meaning of each field, as 6.2
and 6.3 do for FLUTE and TROMBONE), from the lines below.

**Record keys.** None of its own.

**Events and hits** (each `self.ev(kind, who, ...)` and `self.damage(att, vic, dmg, what, x, y, sub)` call, and each
`self.kill_keys(mask, why, by)`, whose events are `why`, with its line):

- 2739: `self.ev('barrage', k, n, salvo)`

<!-- /kind:barrage -->

## 7. The content hash

`Recording::hash()` (Python: `recording.rec_hash`) identifies what happened in the fight. It does not depend on who
wrote the file (`format` and `source` are left out) or on how the zip was packed. It is the first 16 lowercase hex
digits of the SHA-256 of the byte string below.

Building blocks. All integers are little-endian. `u64(n)` is 8 bytes, unsigned.

`V(value)`, the bytes of one JSON-like value:

| Value | Bytes |
| --- | --- |
| `Null` | `4E` (`N`) |
| `Bool` | `54` (`T`) for true, `46` (`F`) for false |
| `Int` | `49` (`I`), then the integer as 8 bytes, two's complement |
| `Float` | `44` (`D`), then the 8 bytes of the IEEE 754 double (its bit pattern as a `u64`) |
| `Str` | `53` (`S`), then `u64(number of UTF-8 bytes)`, then the UTF-8 bytes |
| `List` | `4C` (`L`), then `u64(number of items)`, then `V(item)` for each item in order |

The byte string, in this order:

1. 13 bytes: the 12 ASCII characters `BFREC-HASH-1`, then a newline (`0A`).
2. `V(names)`: a list of strings.
3. `49`, then `seed` as 8 bytes (an `Int`; the seed is unsigned and below 2^63).
4. `V(winner)`: an `Int` or `Null`.
5. `V(t_ko)`: a `Float` or `Null`.
6. `V(f0)`, then `V(fps)`: two `Int`s.
7. `V(corner)`: a list of two-item lists of `Float`s.
8. `V(rules)`, with the rules written as a list of `[key, value]` pairs in ascending key order (byte order of the
   UTF-8 keys, which is the order of a `BTreeMap<String, _>`): `4C`, `u64(count)`, then for each pair `4C`, `u64(2)`,
   `V(key)`, `V(value)`.
9. `V(events)`: a list; each event is the list `[t, kind, who, ...]`, that is `4C`, `u64(2 + fields.len())`,
   `V(Float t)`, `V(Str kind)`, then `V` of each field.
10. `u64(number of arrays)`, then for each array in ascending key order:
    - `V(key)` (a `Str`),
    - for an `F64` array: `66` (`f`), `u64(number of dimensions)`, `u64(each dimension)`, then the data as 8-byte
      doubles in C order, exactly the data bytes of the `.npy` member (NaN payloads included, as stored);
    - for an `I64` array: `69` (`i`), then the same with 8-byte signed integers;
    - for a `Nested` array: `6A` (`j`), then `V(value)` of the whole per-frame list.

`Int` and `Float` are different tags: `1` and `1.0` hash differently, and so do `0.0` and `-0.0`. That is why
section 2 fixes how each is written and read.

### A worked example

A made-up two-frame recording (it only has to exercise the rules):

```
names   ["A", "B"]         seed 7       winner 1      t_ko 2.5      f0 3      fps 30
corner  [[-1.0, 1.0], [1.0, -1.0]]
rules   {"FPS": 30, "swat": true}
events  [[1.0, "hit", 0, 1, 2.5, "blade"], [2.5, "ko", 0, 1]]
arrays  HP  <f8, shape (2, 2):  [[100.0, 100.0], [0.0, 97.5]]
        LV  <i8, shape (2, 2):  [[1, 1], [1, 2]]
        W   JSON:               [[], [[0, 0.5, "wave", null]]]
```

Its byte string is 562 bytes long, in hex (32 bytes a line):

```
42465245432d484153482d310a4c020000000000000053010000000000000041
5301000000000000004249070000000000000049010000000000000044000000
0000000440490300000000000000491e000000000000004c0200000000000000
4c020000000000000044000000000000f0bf44000000000000f03f4c02000000
0000000044000000000000f03f44000000000000f0bf4c02000000000000004c
0200000000000000530300000000000000465053491e000000000000004c0200
00000000000053040000000000000073776174544c02000000000000004c0600
00000000000044000000000000f03f5303000000000000006869744900000000
0000000049010000000000000044000000000000044053050000000000000062
6c6164654c04000000000000004400000000000004405302000000000000006b
6f49000000000000000049010000000000000003000000000000005302000000
0000000048506602000000000000000200000000000000020000000000000000
0000000000594000000000000059400000000000000000000000000060584053
02000000000000004c5669020000000000000002000000000000000200000000
0000000100000000000000010000000000000001000000000000000200000000
000000530100000000000000576a4c02000000000000004c0000000000000000
4c01000000000000004c04000000000000004900000000000000004400000000
0000e03f530400000000000000776176654e
```

SHA-256: `e95b0e26c6a45d73475a5887a4f34d67871d5c09da11383e1f858d9dc5c17891`. The hash is `e95b0e26c6a45d73`.

### The reference recordings

| File | Seed | Winner | `t_ko` | Events | Frames | Hash |
| --- | --- | --- | --- | --- | --- | --- |
| `ep392.bfrec` | 392 | 1 (TROMBONE) | 50.021568627450975 | 522 | 1,394 | `038e1f40fc3373eb` |
| `seed1.bfrec` | 1 | 0 (FLUTE) | 40.63823529411764 | 398 | 1,113 | `8689c1af755ed173` |
| `seed2.bfrec` | 2 | 1 (TROMBONE) | 52.871568627450976 | 505 | 1,480 | `ec9e2ac0d1ca33fc` |
| `seed3.bfrec` | 3 | 0 (FLUTE) | 50.95490196078431 | 510 | 1,422 | `9bff0f539177070e` |
| `seed500.bfrec` | 500 | 0 (FLUTE) | 55.88823529411764 | 528 | 1,570 | `5a00a0b08c4d1bac` |

### The scenario recordings and the fight with no K.O.

Written by the `scenarios` export (`tools/studio_export/scenarios.py`), in the same folder. The scenario fights are
honest fights under rule overrides, so that every event kind, hit sub-kind and ability fires early;
`coverage.json` beside them says where each first fires. The overrides appear in `rules` with the value used:
`early_levels` has `LV_DMG` `[2.0, 5.0]`; `early_low` has `LV_DMG` `[2.0, 5.0]` and `LOW_HP` `92.0`; `early_ko`
has `dmg_scale` `3.0`; `plain` has none. `swapped` has no overrides but TROMBONE in the left corner: `names` is
`["TROMBONE", "FLUTE"]` and `dmg_side` `[0.4, 2.0]`, so every per-fighter array is in that order too (the FLUTE's
baskets are `COB[i][1]`). `no_ko.bfrec` is the lowest seed with no K.O. before `t_max` under the episode's own rules:
`winner` and `t_ko` are `null` and the last frame is the last regular one before 58 s. They hold the same 12 event
kinds and the same 19 keys as the reference recordings.

| File | Seed | Winner | `t_ko` | Events | Frames | Hash |
| --- | --- | --- | --- | --- | --- | --- |
| `no_ko.bfrec` | 4 | null | null | 456 | 1,633 | `52869b4b4b535bd2` |
| `scenarios/early_levels_s1.bfrec` | 1 | 0 (FLUTE) | 39.28823529411765 | 426 | 1,072 | `73aab95a6c47339c` |
| `scenarios/early_levels_s2.bfrec` | 2 | 1 (TROMBONE) | 42.17990196078431 | 455 | 1,159 | `571c237e79e7ec1f` |
| `scenarios/early_levels_s3.bfrec` | 3 | 1 (TROMBONE) | 50.688235294117646 | 517 | 1,414 | `ebdb789f58d7c5ea` |
| `scenarios/early_low_s1.bfrec` | 1 | 0 (FLUTE) | 45.79656862745098 | 450 | 1,268 | `1a6f45574fccfb4e` |
| `scenarios/early_low_s2.bfrec` | 2 | 0 (FLUTE) | 45.34656862745098 | 474 | 1,254 | `5b59ad32b43116c5` |
| `scenarios/early_low_s3.bfrec` | 3 | 1 (TROMBONE) | 45.41323529411765 | 452 | 1,256 | `066bebdb17ee8245` |
| `scenarios/early_ko_s1.bfrec` | 1 | 0 (FLUTE) | 14.879901960784313 | 117 | 340 | `7dae15373cdcb2ea` |
| `scenarios/early_ko_s2.bfrec` | 2 | 0 (FLUTE) | 17.99656862745098 | 147 | 434 | `bb5db53bd0ab1ba6` |
| `scenarios/early_ko_s3.bfrec` | 3 | 0 (FLUTE) | 22.47990196078431 | 191 | 568 | `1c2c47b34c0270df` |
| `scenarios/plain_s7.bfrec` | 7 | 0 (FLUTE) | 57.03823529411764 | 513 | 1,605 | `8d4aab40227ce763` |
| `scenarios/plain_s8.bfrec` | 8 | null | null | 428 | 1,633 | `8b768308801dd33a` |
| `scenarios/plain_s9.bfrec` | 9 | 1 (TROMBONE) | 47.213235294117645 | 422 | 1,310 | `6e0a9423fd198493` |
| `scenarios/swapped_s1.bfrec` | 1 | 1 (FLUTE) | 44.47990196078431 | 453 | 1,228 | `5e305859c8c761c6` |
| `scenarios/swapped_s2.bfrec` | 2 | 0 (TROMBONE) | 51.38823529411764 | 492 | 1,435 | `de3d586f83f4a4c9` |
| `scenarios/swapped_s3.bfrec` | 3 | 1 (FLUTE) | 44.588235294117645 | 433 | 1,231 | `319c0f76b52bc029` |

This hash is the Studio's own. It is not the kit's replay-guard hash (`tools/ballfight_replay_hash.py` prints
`a4235ac9d8e6286b` for seed 392), which is taken over Python `repr` text. The exporter checks that each file holds
everything the replay guard covers by recomputing the guard's hash from the file alone. The guard is compared over
the run's values (`fights.replay_hash_of_values`): a numpy float the kit leaves in an event (ORGAN's `line` event,
whose rope length `fight.py`'s `_to_ropes` returns as a numpy float when a rope cuts the row short) is written as the
float of the same bits, as every float is, and hashed as that float; the kit's own `repr` of it (`np.float64(...)`)
names a Python type no file of one float type can carry. A run without numpy scalars hashes exactly as before.
