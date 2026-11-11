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
  `meta.json` itself, `source`, `rules` and `arrays`.

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

`dmg_side` and `swat_cobras` are attributes only an episode adds, so they can be absent, as in Python's
`vars(rules)`. A recording the Studio makes from a fight file has `dmg_side` unless the file's `handicap` is 1.0 for
each fighter (a file with no `handicap` line; the kit passes no `dmg_side` for an episode without `DMG_SIDE`), and
`swat_cobras` exactly when the file's `pair_rules` set it, with the value set (`false` too: Python's `setattr` lists
an attribute set to false). The fight is the same either way; only the content hash shows it.
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
everything the replay guard covers by recomputing the guard's hash from the file alone.
