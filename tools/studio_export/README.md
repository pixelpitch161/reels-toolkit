# tools/studio_export

The reference export for BALLFIGHT Studio. It reads the Python episode kit (`projects/`, `reelkit/`, `series/`)
without editing it and writes the reference files under `studio/reference/` that the Rust parity tests compare
against. Every file it writes is listed in `studio/reference/manifest.json` with a SHA-256 and what made it.

## The environment

The reference numbers are whatever the kit gives with exactly these versions, on the PC named in the manifest's
`platform` field:

- CPython 3.13.18
- numpy 2.5.3, scipy 1.18.1, pillow 12.3.0 (`requirements.txt`)

On the reference PC (Windows 11) all of it lives in the git-ignored `.studio-tools/` folder at the repo root, made
with uv 0.13.7: `bootstrap/` is a small venv that only holds uv, `python/` the CPython 3.13.18 that uv downloaded,
`uv-cache/` uv's cache, and `py313/` the venv the export runs in. To make the same venv again:

```
uv python install 3.13.18
uv venv .studio-tools/py313 --python 3.13.18
uv pip install --python .studio-tools/py313/Scripts/python.exe -r tools/studio_export/requirements.txt
```

Two environment variables must be set before Python starts:

- `PYTHONHASHSEED=0`: the kit's picture uses Python's string hash, which is salted unless this is set.
- `PYTHONPATH=<repo root>`: the kit imports `reelkit` from there.

## Running it

From the repo root, with the venv's Python (PowerShell shown; in Git Bash use `export NAME=value`):

```
$env:PYTHONHASHSEED = "0"
$env:PYTHONPATH = (Get-Location).Path
.studio-tools\py313\Scripts\python.exe -m tools.studio_export list
.studio-tools\py313\Scripts\python.exe -m tools.studio_export py
.studio-tools\py313\Scripts\python.exe -m tools.studio_export all
.studio-tools\py313\Scripts\python.exe -m tools.studio_export verify
```

- `list` prints the exports in the order `all` runs them.
- `<name>...` runs those exports in the order given.
- `all` runs every export, sorted by its `ORDER` number (50 unless the module sets one), then by name.
- `verify` re-hashes every file in the manifest and exits 1 if one is missing or changed. It only reads files, so
  it runs on any Python. Files under `studio/reference/` that the manifest does not list are printed as notes.
- `adopt <path>...` records reference files that a Rust test blessed (see "The manifest").
- `stages <frame>...` is no reference export: it dumps one frame's buffers after each stage of the renderer into
  the git-ignored cache folder, for finding where a frame first differs (see "`stages`" below). `all` leaves it
  out, and so it does every export a module names in `TAKES_ARGS`: such an export is run alone and gets the rest
  of the command line as a list.

The environment guard: every export, and `list` and `adopt` too, refuses to run unless

- the interpreter is CPython 3.13.18 and was not started with `-O` (the exports check their files with `assert`),
- `PYTHONHASHSEED` is `0` **and really in effect**: the guard looks at the interpreter's own state
  (`sys.flags.hash_randomization`, and `hash('FLUTE')` against its reference value), because under `python -E`,
  `-I` or `-R` the variable still reads `0` while string hashes are random,
- numpy, scipy and pillow are exactly the versions in `requirements.txt`.

Only `verify` is exempt: it computes nothing, it only re-hashes files.

Running an export twice gives byte-identical files. To check: run it, run `git status --short studio`, and nothing
is modified.

## Rules the exports keep

- The kit is read, never written. `common.episode(name, tag)` copies the example's `episode.py` to
  `studio/reference/.cache/episodes/<tag>/<name>/` (git-ignored) and loads that copy with `kit.episode.load`, so the
  caches the kit writes on its own (`check/run_<seed>.pkl`, sprites, `audio.wav`, stems) land in the cache folder.
  `<tag>` is the export's name, so two exports running at once never share a folder.
- No absolute paths, user names or times inside a reference file.
- Text files are UTF-8 without a BOM, with LF line endings. JSON has one fixed layout (`common.dumps`): one-space
  indent, one key per line, a list of plain values on one line, any other list one item per line. Keys are sorted
  unless their order means something.
- Numbers that Rust must match bit for bit are `float.hex()` strings (`common.hexf`), for example
  `0x1.999999999999ap-4` for 0.1: an optional `-`, `0x1.`, 13 hex digits, `p`, a signed decimal exponent. Zero is
  `0x0.0p+0` or `-0x0.0p+0`; a subnormal is `0x0.<13 hex digits>p-1022`. Integers that can pass 2**53 are decimal
  strings (`common.dec`).
- Windows has no `fork`. An export that needs a process pool uses `multiprocessing.get_context("spawn")` with a
  module-level worker function, and each worker loads the episode itself.

## The manifest

`studio/reference/manifest.json` has one line per file, sorted by path: `path` (repo-relative), `sha256`, `bytes`,
`tool` (the module that wrote it), `commit` (`git rev-parse HEAD` at export time), `dirty` (whether `projects/`,
`reelkit/` or `series/` had uncommitted changes then), `python`, `numpy`, `scipy`, `pillow` and `platform`.

`manifest.add(path)` (which the write helpers below call) updates it under a lock file and replaces it in one step,
so exports may run at the same time.
When an export reproduces a file bit for bit with the same tools, its entry is left as it is (only `commit` would
change), so re-running the export on an unchanged kit leaves the working tree clean. Files an export writes outside
`studio/reference/` are listed too, by their repo-relative path.

A reference file that a Rust test blesses (`studio/reference/dmath/bits.json`) is recorded with
`python -m tools.studio_export adopt studio/reference/dmath/bits.json`: the same fields, with `tool` set to
`rust-bless`. Run it again whenever the file is blessed again; `verify` checks it like any other entry.

## Writing reference files

Three helpers write a reference file reproducibly **and list it in the manifest** under the calling module's name:

```python
common.write_json(path, obj, sort_keys=True, register=True)
common.write_npz(path, arrays, register=True)             # arrays: {member name: numpy array}
recording.write_bfrec(run, rules, path, source=None, rect_keys=None, register=True)
```

- `write_json` writes `obj` in the fixed JSON layout. Tuples and numpy values are converted exactly; a float that
  is not finite, an int outside 64 bits and any other type are refused.
- `write_npz` writes a zip of `<name>.npy` members (`.npy` version 1.0, C order, names sorted, the timestamp
  1980-01-01 on every member, deflate level 9: the same settings as a `.bfrec`), so two runs give the same bytes,
  which `np.savez` does not. `np.load(path, allow_pickle=False)` reads it. It takes numpy arrays of dtype `|u1`,
  `|b1`, `<i4`, `<i8`, `<f4` or `<f8` and stores them as they are; lists, object arrays, strings and every other
  dtype are refused, so the dtype in the file is always one the caller chose.
- `register=False` writes without listing: for a scratch file, or when the export checks the file first and then
  calls `manifest.add(path)` itself.

A file written any other way (a PNG saved by Pillow, `common.write_bytes`) needs `manifest.add(path)` afterwards.

## Adding an export

Add a module to this folder. Nothing else changes: the entry point scans the package.

```python
import numpy as np

from . import common

OUT = common.ref('thing', 'data.json')          # studio/reference/thing/data.json


def run():
    common.write_json(OUT, {'about': 'an example', 'values': [common.hexf(0.1)]})          # written and listed
    common.write_npz(common.ref('thing', 'data.npz'), {'c000__out': np.zeros((4, 3), np.float32)})


EXPORTS = {'thing': run}                         # python -m tools.studio_export thing
# ORDER = {'thing': 60}                          # only if it must run after the default 50s under `all`
```

## What each export writes

| Name | Module | Files |
| --- | --- | --- |
| `checks` | `checks.py` | `studio/reference/checks/flute_vs_trombone.json` (its own interpreter; compares its soundtrack with the `audio.wav` of `stems`; draws 673 frames in 5 processes; order 60) |
| `dsp` | `dsp_vectors.py` | `studio/reference/dsp/{butter,filters,resample,fftconvolve,numpy,ufuncs,generators}.npz`, each with its sidecar `.json` (order 40) |
| `fights` | `fights.py` | `studio/reference/fights/flute_vs_trombone/{ep392,seed1,seed2,seed3,seed500}.bfrec` |
| `frames` | `frames.py` | `studio/reference/frames/flute_vs_trombone/{f<frame>.png,frames.json}` (43 frames), and `studio/reference/frames/exceptions.toml` if it does not exist yet |
| `kit` | `kit_vectors.py`, `kit_renderer_vectors.py` | `studio/reference/picture_kit/{config,font,np,camera,geometry,raster,lighting,shadows,lens,fftconv,post,hull}.npz`, each with its sidecar `.json` (about 10 seconds) |
| `knife_edge` | `knife_edge.py` | `studio/reference/fights/knife_edge/{core_s<seed>.bfrec,index.json}`: one core-only fight (the two bags of `striker_bag` against each other, the first 13 s) in which the balls travel side by side exactly R + R apart, so `clash` events are made or not on the last bit of a distance: the real sample of a knife-edge for `bf_sim::parity`; `crates/bf-sim/tests/prefix.rs` reads it (under a second) |
| `lance_bag` | `lance_bag.py` | `studio/reference/fights/lance_vs_bag/{<kind>_<corner>_s<seed>.bfrec,index.json}`: 9 fights of the TROMBONE's lance against a bag (the two bags of `striker_bag`, and a LANCER: the TROMBONE with POWER SLIDE as its only ability; all added to the kit's roster table for the run only), the first 16 s of each, so that only the engine core and the lance act; `index.json` lists them, the three added entries and what the set covers (about 2 s) |
| `models` | `models.py` | `studio/crates/bf-content/data/models/{FLUTE,TROMBONE}.json` |
| `pillow` | `pillow_vectors.py` | `studio/reference/pillow/calls.json` and 18 vector files, each `.npz` with its sidecar `.json` (about 4 minutes: it draws all 1,830 frames in 8 worker processes) |
| `py` | `py_vectors.py` | `studio/reference/py/vectors.json` |
| `pyhash` | `pyhash_vectors.py` | `studio/reference/pyhash/vectors.json` |
| `rng` | `rng_vectors.py` | `studio/reference/rng/vectors.json` |
| `roster` | `roster.py` | `studio/crates/bf-content/data/roster.json` |
| `sprites` | `sprites.py` | `studio/reference/sprites/{FLUTE,TROMBONE}.npz`, each with its sidecar `.json` (about 2 minutes) |
| `stats` | `stats.py` | `studio/reference/fights/flute_vs_trombone/stats.json` (seeds 1 to 4,000, pooled figures; order 55) |
| `scenarios` | `scenarios.py` | `studio/reference/fights/flute_vs_trombone/{scenarios/*.bfrec,no_ko.bfrec,coverage.json}` (reads `stats.json`; order 60) |
| `stories` | `stories.py` | `studio/reference/fights/flute_vs_trombone/stories.json` (the kit's story and score of every recording there; order 70) |
| `striker_bag` | `striker_bag.py` | `studio/reference/fights/striker_vs_bag/{<kind>_<corner>_s<seed>.bfrec,index.json}`: 12 fights of the FLUTE against a bag (a fighter with no weapon, added to the kit's roster table for the run only), the first 10 s of each, so that only the engine core and the striker act; `index.json` lists them, the two bag entries and what the set covers (about 4 s) |
| `title` | `frames.py` | `studio/reference/frames/flute_vs_trombone/title/{f<frame>.png,noise.png,title.json}` (18 cards) |
| `stems` | `stems.py` | `studio/reference/sound/flute_vs_trombone/{audio.wav,stems.npz,premaster.npz,rms.json,master_case.npz}` (its own interpreter; reads what `voices` wrote; order 55) |
| `voices` | `voices.py` | `studio/reference/sound/{pp_noise_start.json,flute_vs_trombone/{note_calls.json,first_notes.npz},voices/{FLUTE,TROMBONE,helpers}.npz,voices/scalars.json}` (its own interpreter) |
| `kit_fx` | `kit_fx_vectors.py` | `studio/reference/picture_kit/fx_{timing,bursts,warp,particles,marks,moments}.npz`, each with its sidecar `.json`: the effects kit, `reelkit.fx` (about 40 s: it renders some 50 frames to record the real calls; see its docstring for the cases) |

### `roster`: the fighters' numbers

`{"about", "format": 1, "fighters": [...]}`: the 20 fighters the kit has ported, in the order of
`projects/ballfight/fighters.py`. Each is an object with `name` first, then the entry's own keys in their own order
(`color`, `dark`, `corner`, `view`, `weapon`, `abilities` with `unlock1`, `unlock2` and `low`, `pp`, `register`,
`idiom`; `pitch` where an entry has one). Every number is kept exactly: an int stays an int, a float reads back to
the same bits, a tuple becomes a list. The file is read back and compared with the table before it is listed in the
manifest. This is the one file here whose key order means something, so its keys are not sorted.

The two retired fighters are never written and the export stops if one would be. One string equal to a retired
name is expected and is not a fighter: ELECTRIC's `pp`, the name of the Pixel Pitch voice and model it uses.

`recording.py` has no export of its own: it is the Python side of the recording container (`write_bfrec`,
`read_bfrec`, `rec_hash`), used by `fights` and by later exports.

### `fights`: reference recordings

The example episode `flute_vs_trombone` fought by the kit (`story.run_seed`) for its own seed 392 and for seeds 1,
2, 3 and 500, each stored with the episode's rules as a `.bfrec` file. The container, every field, every event kind
and the content hash are specified in `studio/docs/recording-format.md`; that document is the contract the Rust
reader is written from, and it states each file's hash. The export stops with a message if the document no longer
states the hashes it computes.

After writing a file the export reads it back and checks it against the run value for value and type for type,
and it recomputes the kit's replay-guard hash from the file alone: for seed 392 that is `a4235ac9d8e6286b`, the
same line `python projects/ballfight hash flute_vs_trombone 392` prints.

The zip bytes also depend on the zlib that Python carries (1.3.2 in the reference environment). The content hash
does not.

### `rng`: numpy's `default_rng`

For `bf_core::rng`. `{"about", "format": 1, "numbers", "seeds": [{"seed", "init", "streams"}], "resume"}`.

- `seed` is a JSON integer (`default_rng(seed)`: 0, 1, 2, 392, 2**40 + 7) or a list (`default_rng([392, 44, 3588])`,
  `default_rng([1, 2, 3])`: the list seeds the engine makes mid-fight).
- `init` shows how the seed becomes a generator: `seed_sequence_u64` (the four 64-bit words
  `SeedSequence(seed).generate_state(4, np.uint64)` hands PCG64), the fresh generator's `state` and `inc`, and its
  first eight raw 64-bit outputs `raw_u64`. All decimal strings.
- `streams` maps a name to `{"call", "op", "args", "calls", "values", "end"}` (and `"size"` for one array call).
  **Every stream starts from a fresh `default_rng(seed)`.** `call` is the Python call as text; `op` and `args` say
  the same for a program; `calls` is how many times it was made; `end` is the bit generator afterwards: `state`,
  `inc`, and the buffered half of a 64-bit output (`has_uint32`, `uinteger`) that 32-bit draws leave behind.

| Stream | Call | Values |
| --- | --- | --- |
| `random` | 64 x `rng.random()` | f64 |
| `uniform_m1_1`, `uniform_5_9` | 64 x `rng.uniform(-1, 1)`, `rng.uniform(5, 9)` | f64 |
| `normal_0_1`, `normal_2_0p5` | 256 x `rng.normal(0, 1)`, 64 x `rng.normal(2, 0.5)` | f64 |
| `normal_0_1_size1000`, `uniform_0_1_size1000` | one call with size 1000 | f64 |
| `integers_7`, `integers_1000`, `integers_2p40` | 64 x `rng.integers(high)` | int |
| `integers_1`, `integers_2p31_plus_1`, `integers_2p32`, `integers_2p32_plus_1` | corners of the bounded-integer code | int |
| `integers_3x2p61` | 64 x `rng.integers(3 * 2**61)` | decimal strings (so is its `args`) |
| `permutation_2`, `_5`, `_16`, `_100` | one `rng.permutation(n)` | int |
| `choice_m1_1`, `choice_5` | 64 x `rng.choice(list)` | the picked values, and their positions in `indices` |
| `interleaved` | 32 rounds of `uniform(-1, 1)`, `normal(0, 1)`, `random()`, `integers(5)`, `permutation(4)` on ONE generator | a row per round: three f64, the integer, then the 4 permutation values |

The corners: a bound of 1 draws nothing (the state does not move); up to 2**32 the draws are 32-bit, and numpy
keeps the unused half of each 64-bit output for the next 32-bit draw (low half first); 2**32 takes the raw 32
bits; 2**31 + 1 rejects about half its draws and 3 * 2**61 about a quarter, so the rejection loops run.

`resume`: after 1,000 x `rng.normal(0, 1)` from seed 77, the generator's `state` and `inc` (decimal strings),
`has_uint32` (0) and `uinteger`, and under `next` the 64 normals that follow.

### `pyhash`: Python's string hash

For `bf_core::pyhash`. `{"about", "format": 1, "numbers", "rows", "non_ascii"}`. A row is `[string, hash,
hash % 997]`: the hash is `hash(string)` under `PYTHONHASHSEED=0` as a decimal string (signed 64-bit), and
`hash % 997` is Python's (0 to 996), which is what `reelkit/ui.py` uses. `rows` has 500 ASCII strings: the empty
string, every fighter, ability and weapon name in `fighters.py` except the two retired fighters, the caption lines
with and without their `*` marks, and seeded random printable ASCII of length 1 to 40. `non_ascii` has 12 more
strings beyond ASCII: CPython hashes the string's internal storage (one byte per character below U+0100, else two
bytes little-endian below U+10000, else four), not its UTF-8 bytes.

### `py`: Python number semantics

For `bf_core::py`. `{"about", "format": 1, "numbers", "rows_per_case": 2000, "cases": {name: {"expr", "inputs",
"output", "rows"}}}`. A row is the inputs in the order of `inputs`, then the output. Float inputs and `f64` outputs
are `float.hex()` strings and are meant to be compared bit for bit, the sign of zero included; `int` outputs are
JSON integers.

| Case | Python expression | Output |
| --- | --- | --- |
| `round` | `round(x)` | int (no minus zero: `round(-0.3)` is `0`) |
| `round_1`, `round_2`, `round_3`, `round_5` | `round(x, n)` | f64 |
| `np_round_2` | `np.round(x, 2)` | f64 |
| `rint` | `np.rint(x)` | f64 (keeps the sign of zero) |
| `mod`, `floordiv` | `a % b`, `a // b` on floats | f64 |
| `wrap_pi` | `(a + math.pi) % (2 * math.pi) - math.pi` (`fight.py` `_wrap`) | f64 |
| `imod`, `ifloordiv` | `a % b`, `a // b` on ints | int |

### `pillow`: what Pillow draws

For the Pillow-exact primitives of `bf-picture`. The renderer draws its polygons, lines, crowd, shadow masks and
bloom with Pillow 12.3.0, so the Studio has to give the same pixels.

`calls.json` is the survey that decides what must be covered: every `ImageDraw` and `Image` call the kit makes
while it draws all 1,830 frames of the example episode, one row per call, image mode, drawing mode and call site
(`site`; `via` names the caller when the site is a method of `reelkit.raster.Frame`), with the count, the form and
Python types of the coordinates, the smallest and largest point count and coordinate, whether any coordinate had a
fraction or lay outside the image, every (fill kind, outline kind, width) combination seen, the alpha range, and
`cases`: the vector files that reproduce the row. The export stops if the renderer makes a call that no case
reproduces. The survey starts after both balls' sprites and the title close-ups are built, because the Pixel Pitch
modules the bridge imports for the models paint posters for their own videos as they load; no frame shows those.

| File | Cases |
| --- | --- |
| `polygon` | 208: `ImageDraw.Draw(im).polygon(xy, fill=...)` on `L`, `RGB`, `F` (the depth buffer, float32) and `RGBA` images |
| `polygon_outline` | 200: `polygon(xy, fill=..., outline=...)` and the outline alone (the ink stars of `reelkit.fx.starburst`, on `RGBA`) |
| `line` | 396: `line(xy, fill=..., width=...)` on `RGB`, `F` and `L`: no width given, and widths 1 to 10 (the cobras' bodies go up to 8); polylines of up to 19 points |
| `ellipse` | 208: `ellipse(xy, fill=...)`, `ellipse(xy, outline=...)` and both, on `RGB`, `L` and `RGBA` |
| `rectangle` | 204: `rectangle(xy, fill=...)`, a few outlined, on `RGB` and `L` |
| `point` | 200: `point(xy, fill=...)` on `RGB`, `L` and `F` |
| `blend_polygon`, `blend_line`, `blend_ellipse`, `blend_rectangle`, `blend_point` | 200 or more each: the same calls through `ImageDraw.Draw(rgb_image, 'RGBA')`, which blends the ink into the picture by its alpha (particles, speed lines) |
| `polygon_rounding` | 57: filled polygons (plain on `L`, `RGB`, `F`, `RGBA`, and blended) with an edge that crosses a row at 0.49999997, the largest float below one half. `Draw.c` rounds a span's start with `floor(f + 0.5F)`, a C float sum, which is 1.0 there: the span starts at column 1, where a double sum would start it at column 0. Edges from (0, 0) with slope 1/82, 1/94, 1/110, 1/122, 1/164 and 1/166 do this on rows 41, 47, 55, 61, 82 and 83 (`args.row`); so does one built for it on row 1. The export checks on every case that the row's column 0 is untouched and its column 1 drawn |
| `nonfinite` | 201: the drawing calls with a coordinate that is NaN, infinite or beyond a C int, on a 16 x 12 image: boxes with such a corner (rectangles and ellipses, filled, outlined, blended), ellipses up to 10^7 pixels across, wide and thin lines with such an end, polygons with such a vertex, points. In `args` those values are `"nan"`, `"inf"` and `"-inf"` |
| `gaussian_blur` | 202: `im.filter(ImageFilter.GaussianBlur(radius))` on `L` (0.7: the shadow masks) and `RGB` (3.0: the bloom), and other radii |
| `resize_nearest`, `resize_bilinear`, `resize_bicubic` | 200 each: `im.resize(size, filter)` on `L`, `RGB` and `RGBA`. The renderer itself only resizes `RGB` with `BILINEAR` (the bloom: down by 4, up by 4) |
| `as_used` | 23 whole calls captured from `kit.picture.render` at full size: four canvas shadow masks (63 to 70 polygons, then the 0.7 blur), four blooms (the source, the quarter-size picture, its blur, the result), four spot-beam hulls, nine ink starbursts, two title-card cracks |

The seeded cases draw on canvases of at most 96 x 96. A case's `args` hold the image `mode` and `size` (width,
height), its `background` (the one colour it starts with) or the array `in`, `xy` exactly as passed (`xy_form` is
`pairs` for a list of `(x, y)` or `flat` for `x0, y0, x1, y1, ...`), and the call's keyword arguments exactly as
passed: a keyword that is absent was not passed. Floats are `float.hex()` strings. `kind` names what the case is
about: `quad`, `circle`, `star`, `int` (whole-number vertices), `outside_partly`, `outside_far`, `outside_all`,
`two_points`, `collinear`, `same_point`, `bowtie`, `pentagram`, `staircase`, `horizontal_runs`, `flat_top`,
`sliver_subpixel`, `sliver_thin`, `negative_fraction`, `half_pixel` and so on. Arrays are (height, width) for `L`
and `F`, (height, width, 3) for `RGB`, (height, width, 4) for `RGBA`.

What the cases pin down, from Pillow's source (`_imaging.c`, `Draw.c`, `Resample.c`, `BoxBlur.c`):

- Every coordinate becomes a C `int` by truncation toward zero before anything is drawn (`-0.7` is column 0, not
  -1; `10.99` is 10), for every drawing call.
- A line of width 1 (or with no width given) is a Bresenham line through the truncated vertices; a wider line is
  one filled quadrilateral per segment, with no joints. A filled polygon is an edge-list scan fill with Pillow's
  own rounding at each edge; its outline is 1 px lines from vertex to vertex, drawn after the fill.
- An `F` image takes a Python float as its ink and stores it as float32.
- Through `Draw(image, 'RGBA')` the ink is blended into every pixel it lands on by its alpha. A filled polygon
  blends each pixel once even where its edges cross; the separate pieces of one call (the segments of a wide line,
  a point given twice, the two sides of a flattened outline) blend again where they overlap.
- `resize` works on premultiplied colour for `RGBA` (every filter but `NEAREST`).
- `GaussianBlur` is three passes of an extended box blur, and reads its radius as a C float.
- Pillow does not refuse a NaN or infinite coordinate (its box check `x1 < x0` is false for a NaN). It converts it
  to a C `int`, which on the reference PC gives the lowest `int`, and the arithmetic that follows wraps. The C
  standard leaves both undefined, so `nonfinite` records the reference build's answers: a rectangle with a NaN
  left edge is filled from column 0; a thin line to a NaN end paints a run to the image's edge; a filled ellipse
  with a NaN corner draws nothing. Pillow walks every pixel on the way, on the image or not: a thin line with such
  an end takes it 1.5 to 2.7 s, a rectangle outline with a NaN row 5.4 s, and an ellipse outline with one NaN
  corner did not return within 40 s, so `nonfinite` holds only calls that Pillow finishes (the export spends
  about half a minute on them).

### `kit`: the renderer kit's own helpers

For `bf-picture`'s ports of the `reelkit` picture modules. One file per group in `studio/reference/picture_kit/`;
a case is one Python expression (`call`), its scalar arguments (`args`) and its arrays. `kit_vectors.py` holds the
three groups of the table below and the export itself; the renderer kit's nine groups (`camera`, `geometry`,
`raster`, `lighting`, `shadows`, `lens`, `fftconv`, `post`, `hull`) are in `kit_renderer_vectors.py`, whose
docstring lists them.

| File | Cases |
| --- | --- |
| `config` | 12, from `reelkit.config`: `BAYER`, `BAYER_FULL`, `XX` and `YY` as they are (the last three are (640, 360): height, width), and `ss`, `ease`, `lerp`, `mixc`, `whole_period`, `loop_bpm`, `safe_x1` and `safe_overshoot` on fixed inputs. A scalar helper's case holds one input array per argument (named as the argument) and `out`: row `k` of `out` is one Python call on row `k` of the inputs, with Python floats (ints for the two safe-zone helpers). The sidecar's `constants` holds `W`, `H`, `UP`, `FPS`, `DUR` (a `float.hex()` string), `NF`, `SAFE`, `SAFE_BUTTONS` and `SAFE_CX` |
| `font` | 265, from `reelkit.font`: all 58 glyphs as one (58, 7, 5) bool array, in the order of the sidecar's `glyphs` string; `text_mask(s, scale)` for 119 strings and scales (the whole table, lower case, the kit's own words, the empty string, characters the font does not have, and characters whose upper case is two characters or a letter of the table); `dilate(m, r)` for radii 0 to 4 on 13 masks; `stamp` 80 times on a 64 x 40 noise frame, inside it, across every edge and outside, with each of `outline`, `shadow` and `alpha` passed and left out. The sidecar also holds `ADV` |
| `np` | 85 numpy calls the renderer leans on: `np.clip` with two bounds (float64 and float32) and with one, `np.maximum` and `np.minimum`, `astype` from floats to `uint8`, `int32` and `int64`, `np.clip(x, 0, 255).astype(np.uint8)`, the kit's two floor dithers with its own constants and the Bayer table, `np.roll` on 1-D, 2-D and 3-D arrays, and `np.nonzero` |

In `font`, masks are bool arrays (height, width). A `stamp` case has `in` (the frame before), `mask` and `out`; its
`args` hold `x`, `y`, `color` and the keywords exactly as passed (an absent keyword was not passed; `outline` null is
`outline=None`; `alpha` is a `float.hex()` string), and its `call` names the keywords too.

What the `np` cases pin down that a port would not guess:

- `np.clip(x, lo, hi)` keeps a NaN, and keeps an `x` that equals a bound as it is (`np.clip(-0.0, 0, 1)` is `-0.0`).
- A float cast to `uint8` drops the fraction and keeps the low 8 bits of the whole number: it wraps, it does not
  clamp (`-1.5` is 255, `256.7` is 0). NaN, the infinities and values beyond 32 bits become 0; cast to `int32` or
  `int64` they become the type's lowest value. numpy leaves such casts to the processor, so these are the reference
  PC's answers; the kit only relies on casts of values that fit.
- `np.clip(x, lo, None)` is `np.maximum(x, lo)`. For `-0.0` against `0.0` its answer depends on where the element
  sits in the array (the vectorised part of the loop and its tail differ), so the one-bound cases hold no zero.

### `models`: the instrument meshes

`studio/crates/bf-content/data/models/{FLUTE,TROMBONE}.json` are data the Studio ships with, not test vectors:
`pp_bridge.model_faces(name)`, the Pixel Pitch model centred on its bounding box and scaled to fit a unit sphere
(112 faces for FLUTE, 573 for TROMBONE). `{"about", "format": 1, "name", "source", "fields", "note", "faces"}`; a
face is `{"P": [[x, y, z], ...], "n": [x, y, z], "rgb": [r, g, b], "two", "emit", "gid", "spec"}`: 4 to 12
vertices (y up), its unit normal, its colour (floats 0..255, as the bridge gives them), whether it is drawn from
both sides, whether its colour is used unshaded, the group id (the sprite renderer treats 1 as ground and
everything else as the instrument) and the specular strength. Floats are written by their shortest round-trip repr
and read back to the same bits. The faces stay in the order the model was built in: the sprite renderer draws them
in that order and a later face wins only where it is strictly nearer.

The export reads each file back, compares it with the bridge's output value for value, and draws three sprites
from the file alone that must equal `sprites.render_sprite`'s.

### `sprites`: the instrument inside each ball

For `bf-picture`'s sprite renderer and the looks' own sprites. One case per sprite, RGBA `(size, size, 4)`.
`args.group` says which call:

| Group | Call | Cases |
| --- | --- | --- |
| `picture` | `kit.picture.sprite(k, radius, yaw, roll)` | radius 12, 16, 22, 30; yaw 0, 6, ... 354; roll 0 and the roll of the fighter's view (32 for FLUTE, 30 for TROMBONE): 480 per fighter |
| `episode` | the same call | every (radius, yaw, roll) the 1,830 frames ask for that `picture` lacks: 23 for FLUTE, 17 for TROMBONE. The frames use radii 13 to 19 only |
| `title` | `title.big_sprite(k)` | the VS card's close-up (radius 44). `args.drawn_by` says how it is made: `closeup` for FLUTE (its look's own close-up function, called with the radius alone, so the case has no yaw or roll), `model` for TROMBONE (`sprites.render_sprite` at its view's yaw and roll) |
| `model` | `sprites.render_sprite(name, radius, yaw, roll)` | FLUTE only (480): its look draws its own pixel-art flute, so `picture` never shows its mesh; these give the sprite renderer something to test that mesh against |

For TROMBONE `kit.picture.sprite` is the model renderer (`sprites.render_sprite` with `piano_lowpoly.render_geo`,
`tri` and `outline`), size `2 * radius + 7`. For FLUTE it is `flute_sprite` of `kit/fighters/flute.py`, size
`2 * radius + 5`. The sidecar's top level has `drawn_by` (`look` or `model`), `view`, `sprite_k`, `rolls` and
`episode_keys`, the (radius, yaw key, roll) triples the frames ask for.

A trap the port must copy: `kit.picture.sprite` rounds the yaw to a multiple of 6 (`yk`) and caches under
`<pp>_<r>_<yk % 360>_<roll>`, but calls the look's sprite with the unwrapped `float(yk)`. FLUTE's yaw swings
between -18 and 18, and `flute_sprite` at -6 is not the same picture as at 354 (its length goes with
`cos(1.6 * yaw)`), so the `episode` cases for FLUTE have negative yaws and the `picture` cases at 342 to 354 are
sprites the frames never show.

Every `picture`, `episode` and `model` case is the call on an empty sprite cache. A `title` case is
`title.big_sprite(k)` with the title module's in-memory cache emptied first; for a model close-up `big_sprite` may
still read its own file cache, so the export compares every close-up with a fresh draw.

Which sprites the frames ask for depends on where the balls are, so the export first proves the committed
recording is the fight the kit draws (as `frames` does; the sidecar's `recording` has the file and its content
hash). It then works out what the frames ask for from the kit's state functions (`sprites.asked`) and proves it
against real renders of 72 frames with `kit.picture.sprite` wrapped; it clears the kit's sprite cache when it is
done, so a later export in the same process cannot meet a sprite of another yaw.

### `frames`: golden frames

`kit.picture.render(frame)` for 43 frames of the example episode (seed 392), each an RGB PNG at the internal
360 x 640, written by Pillow with its default settings.

The Studio draws its frames from the committed recording `studio/reference/fights/flute_vs_trombone/ep392.bfrec`,
while the kit draws from a run of its own. So before anything else the export reads that file
(`frames.check_recording`) and stops unless it is the kit's run: the same seed, names, corners, first fight frame,
winner, K.O. time and rules, every event value for value and type for type, every per-frame array bit for bit. If
the recording is ever exported again under other rules, `frames` fails with the first difference instead of
leaving golden frames of another fight. `frames.json`'s `recording` names the file, its content hash (`hash`: what
`Recording::hash` gives in the Studio), the seed, the winner, the K.O. time and the event and frame counts. `title`
and `sprites` make the same check and carry the same entry.

The frames are picked from the recording's events by fixed rules (`frames.select`), and `frames.json` lists them:

`{"about", "tool", "episode", "seed", "fighters", "winner", "call", "fps", "size", "mode", "nf", "recording",
"timeline", "rule_notes", "absent", "tag_notes", "tags", "frames"}`. A frame is `{"frame", "t", "file", "rules",
"reason", "tags"}`: its number, its time `frame / 30`, the PNG's name, the ids of the rules that picked it, the
reason in words, and its tags. `timeline` holds the times the rules read (`t_fight`, `t_ko`, `t_solo`, ...) as
`float.hex()` strings. A frame "at" or "after" a time is the first frame whose time is at or after it.

| Rules | Frames |
| --- | --- |
| `frame_0`, `frame_13`, `frame_27` | the title card: name A slamming in, the frame before VS slams in (it does on 14), name B slamming in |
| `doors_last_closed`, `doors_mid` | 43, the last frame with the card shut, and 48, the doors half open |
| `count_3`, `count_2`, `count_1`, `count_fight` | the first frame of each count-in word: 54, 68, 81, 108 |
| `<FIGHTER>_unlock1_0.2s`, `_0.6s`, `_unlock2_...`, `_low_...` | 0.2 s and 0.6 s after the first use of each of the fighter's three abilities (12 frames) |
| `<FIGHTER>_weapon_hit` | on its first weapon hit (`fang`: 179, `blade`: 135) |
| `<FIGHTER>_low_hp` | the first frame with its HP below 25 (1269, 1287) |
| `swat` | on the first cobra swat (229) |
| `ko`, `ko_0.2s`, `ko_0.6s`, `ko_1.5s` | the K.O. frame 1501 and 0.2, 0.6 and 1.5 s after (1.5 s: the WINS caption) |
| `solo_mid`, `comment`, `reassembly_mid`, `glide_mid` | the scripted tail: 1642, 1629, 1750, 1792 |
| `frame_1828`, `frame_1829` | the last two frames (the doors shutting; the loop seam) |
| `extra_*` | seven frames beyond the brief's list: an ability caption and the K.O. caption on the frame they land (the scale punch), the WINS caption's second line landing, each fighter's unlock sweep, the bell on the canvas, the doors closing |

`absent` names what this fight cannot show: there is no `parry` frame, because a parry is the lance cutting a
projectile and neither of these fighters shoots one (the recording has no `parry` event). What is absent is judged
on the recording's events by code of its own (`frames.cannot_have`), not by the selection: the export stops if the
two disagree, if any other required frame is missing, if there are fewer than 40, or if a PNG does not read back
as the array drawn.

`tags` (each explained under the top-level `tags`) say what a frame shows, read off the kit's own state
functions: the phase (`rev`, `fight`, `hitstop`, `tail`, `solo`, `reassembly`, `glide`), the card (`card`,
`card_closed`, `doors`), the shared layers (`hud`, `hud:low_hp`, `hud:sweep`, `hud:pill`, `hud:flash`,
`hp_numbers`, `countdown`, `caption:*`, `burst`, `fringe`, `moment`, `particles`, `marks`, `flares`, `sparks`,
`shimmer`, `glass_shards`, `flashbulbs`) and each fighter's own layers (`FLUTE:basket`, `FLUTE:cobra`,
`FLUTE:strike`, `FLUTE:swatted`, `FLUTE:dance`, `FLUTE:haste`, `FLUTE:wave`, `FLUTE:shards`, `TROMBONE:thrust`,
`TROMBONE:trail`, `TROMBONE:wave`, `TROMBONE:slow`, `TROMBONE:haste`, `TROMBONE:slam_decal`, `TROMBONE:bell`,
`TROMBONE:solo`). The plain tags `FLUTE` and `TROMBONE` mean "an action layer of that look is drawn". Every frame
also shows something only each look draws while its ball is whole (`FLUTE:sprite`, the look's own pixel-art flute
and glass; `TROMBONE:slide`, the lance at rest), so no golden frame can be drawn with a default look for either
fighter.

A frame is a pure function of its number: the export draws three of them a second time, last to first, and
compares.

`studio/reference/frames/exceptions.toml` is the list of golden frames the Studio may miss by the documented bar.
The export writes it once, with an empty list, and records it with `adopt`; it never overwrites it. After an edit
run `python -m tools.studio_export adopt studio/reference/frames/exceptions.toml`.

### `title`: the VS title card alone

The card is 94 % opaque over a dimmed arena, so the full frames cannot test the arena under it or the card on
its own. `title/noise.png` is one fixed image (`np.random.default_rng(20261110).integers(0, 256, size=(640, 360,
3), dtype=np.uint8)`) and each `title/f<frame>.png` is `title.draw(noise, frame / 30)`. `title.json` lists the 18
cases: `{"id", "call", "args": {"frame", "t"}, "file", "rule", "reason", "expect": {"door_offset", "shake"}}`,
with `t`, `title.door_offset(t)` and `title.shake(t)` as `float.hex()` strings. The card takes only the two
fighters' names from the fight (nothing on it depends on who wins); `title.json` still carries the `recording`
entry of the checked file.

| Rules | Frames |
| --- | --- |
| `slam_<word>_<n>` | each word's first frame and 2 and 5 frames after: 0, 2, 5 (FLUTE), 14, 16, 19 (VS), 27, 29, 32 (TROMBONE) |
| `hold` | 41: every word landed, no shake, the doors shut |
| `opening_0` .. `opening_3` | 46, 48, 50, 52 of the doors opening (44 to 52) |
| `closing_0` .. `closing_3` | 1824, 1826, 1827, 1829 of the doors closing (1822 to 1829; the last is shut again, with name A as on frame 0) |

That is 18 cards where the brief counts 16: its own list (three per word, the hold, four and four) adds up to 18.

### `stages`: one frame, stage by stage (not committed)

```
.studio-tools\py313\Scripts\python.exe -m tools.studio_export stages 150 600 1500
```

writes `studio/reference/.cache/stages/flute_vs_trombone/f<frame>.npz` and `f<frame>.json` for each frame named
(about 7 MB and 5 s a frame). The folder is git-ignored and nothing in it is listed in the manifest. The files
follow the vector convention: the sidecar's `stages` lists the stage names in pipeline order and each case is one
stage (`id` is its name, `call` the hook that captured it, `what` says what the buffer is, `dtype` its arrays'
dtypes). `rgb` arrays are (640, 360, 3), `depth` and `map` arrays (640, 360), each in the dtype the renderer holds
it in at that point.

| Stage | The buffer | Arrays |
| --- | --- | --- |
| `floor` | the arena floor, lit per pixel and dithered | `rgb` float32, `depth` float32 |
| `arena` | after the stands, the crowd and the LED boards | `rgb` uint8, `depth` |
| `skirt` | after the ring platform's skirt | `rgb` uint8, `depth` |
| `canvas` | the canvas, lit per pixel with its shadows and the looks' decals, dithered | `rgb` float32, `depth` |
| `painter` | after the back post and ropes, the balls, the looks' items, the side posts, the front ropes and post | `rgb` float32 (whole numbers), `depth` |
| `over_ring` | after the looks' over-ring effects and the K.O. shards | `rgb` float32 |
| `sparks` | after the hit sparks | `rgb` float32 |
| `particles` | after the particles: what the lens gets (once clipped and cut to bytes) | `rgb` float32, `depth` |
| `coc` | the circle of confusion per pixel | `map` float64 |
| `lens` | after the depth of field | `rgb` float64 |
| `shimmer` | after the looks' heat shimmers | `rgb` float64 |
| `glows` | after the glow sprites | `rgb` float64 |
| `beam` | after the spot's beam and the can, clipped to 0..255 (the bloom's input) | `rgb` float64 |
| `bloom`, `grade` | after each | `rgb` float64 |
| `scrim` | after the vignette and the top scrim, clipped | `rgb` float64 |
| `post_fx` | after the looks' crisp overlays, the fringe, the ability moments and the bursts | `rgb` float64 |
| `countdown` | cut to uint8, with the count-in drawn | `rgb` uint8 |
| `hud` | after the HP numbers, the fight HUD and the captions | `rgb` uint8 |
| `title` | after the VS title card: the frame | `rgb` uint8 |

The dtypes above are what frames 150, 600 and 1500 give; each sidecar states its own dump's.
`Frame.set_array` clips to 0..255 and truncates to uint8 for the PIL canvas, so `floor`, `canvas` and `particles`
are the floats handed to it and the stage after each starts from whole numbers. Nothing in the kit is edited:
`stages.tap` wraps the functions `render` calls (and `Frame.set_array` and `Frame.array`) for one render and puts
them back. Every dump is checked before it is kept: each stage fired, in this order; the last stage is the frame
`render` returned; the same frame drawn without the taps is identical; every array reads back without pickle.
Like `frames`, the command first proves the committed recording is the kit's run, and each sidecar's `recording`
names it with its content hash.

### `dsp`: signal-processing vectors

For `bf-sound`'s DSP layer (filters, resampling, convolution, the `reelkit.audio` generators). What the episode asks
for is learned by wrapping `scipy.signal.butter` and the `reelkit.audio` generators at run time while
`kit.music.main()` builds the `flute_vs_trombone` soundtrack and the band and loudness checks run; the vectors are
then made by calling the real functions. The export runs in its own interpreter (`dsp_vectors.fresh`), because
building the soundtrack advances the Pixel Pitch voices' shared noise generator. Every array is `float64`; a scalar
result is a 1-element array and also a `float.hex()` string in the case's `result`. Floats in `args` are
`float.hex()` strings; `call` shows the same call with decimal numbers for a reader.

| File | Cases |
| --- | --- |
| `butter` | Every distinct `scipy.signal.butter` design asked for (311 for this episode): `output="sos"` from `reelkit.audio.sos` (`hp`, `lp`, `bp`, order 2, `fs=48000`) and from the band check (order 4 band-pass), and one `output="ba"` design, the series' own `lp(x, 2500)` at 44,100 Hz (the flute's chiff). `args`: `N`, `Wn` (one or two cutoffs), `btype` as the caller wrote it, `fs` (null: `Wn` is a fraction of Nyquist), `output`. `requests` counts the calls, `used_by` names the kit functions |
| `filters` | `sosfilt(sos, noise, axis=0)` for 14 of those designs on the fixed input `noise` (`default_rng(4800).normal(0, 1, 4800)`), one on the stereo `noise2`; `lfilter` for the series' low-pass, the master's envelope follower `([0.002], [1, -0.998])` and the two K-weighting stages of the loudness check |
| `resample` | `resample_poly(x, 160, 147)` on noise, a spike, a real 44,100 Hz trombone note and a 100-sample input; `firwin` (the 3,201-tap low-pass scipy designs) and `h` (what it hands to `upfirdn`) are stored with them |
| `fftconvolve` | Two pairs: 1,000 against 300 samples, and 12,000 against a 48,000-sample tail shaped like the reverb's |
| `numpy` | `np.interp` (the glide warp with `right=0.0`, the tape stop, sparse points), `np.percentile(x, 99.97)`, `np.convolve(f ** 2, ones(w) / w, "same")`, `np.cumsum`, `np.linspace`, the sums and means the sound code takes (numpy adds a long 1-D array pairwise), the peak, the 16-bit conversion of `reelkit.audio.write`, and `scipy.signal.find_peaks(x, height=6.0, distance=4)` on the input `reelkit.levels.profile` gives it for the episode's `audio.wav` and on a synthetic one with flat tops |
| `ufuncs` | `np.sin`, `np.cos`, `np.exp`, `np.tanh`, `np.sqrt`, `np.log10` and the powers the sound code takes, on 2,048 fixed inputs each; cases whose `call` starts with `python:` are the Python-scalar maths (`math.cos`, `math.sin`, `math.exp`, `float ** float`) one value at a time |
| `generators` | Each `reelkit.audio` generator at its default arguments (`stand_in` gives the values used for arguments without a default) and at each argument set the soundtrack uses. `signatures` holds every generator's parameters and defaults; `episode_calls` lists every argument set used, with its count, callers and the ids of its cases |

Two things `generators` does to stay small. `thud` is called with 229 argument sets that differ mostly in `seed`:
for each `(dur, f0, f1)` only the first two seeds used have vectors (`episode_calls` still lists all of them). And
`reverb` and `tape_stop` take a whole bus, so their parameter sets are applied to stand-in buffers stored in the
file: `impulse` (the output is then the first second of the impulse response) and `burst` for `reverb`, `tape`
with `t0 = 0.05` for `tape_stop`.

`dsp_vectors.check()` reads the files back and recomputes the scipy and numpy results from the stored inputs; the
export runs it before it lists anything in the manifest. `dsp_vectors.py` also holds what the other sound exports
share: `Cases` (one vector file and its sidecar, written and read back) and `fresh` (run an export in its own
interpreter).

### `voices`: the Pixel Pitch voices, the `kit.music` helpers and the shared noise stream

For `bf-sound`'s voices and music helpers. **Every Pixel Pitch voice takes its noise from one generator**,
`drums60_lowpoly._rng = np.random.default_rng(77)`, which every `nz(n)` call advances (the FLUTE's breath is
`lp(nz(n), 2500)`; the TROMBONE draws none). So a note's samples depend on every noisy note made before it in the
same interpreter. The export therefore runs in its own interpreter and stores the generator's state (PCG64 `state`
and `inc`, decimal strings) with everything.

| File | What |
| --- | --- |
| `sound/pp_noise_start.json` | The generator after all imports and before the first note. It is the state of a fresh `default_rng(77)`: importing the kit and the two voices' modules draws nothing |
| `sound/flute_vs_trombone/note_calls.json` | Every first-time `kit.music.note(name, midi, dur, cap, vel)` call while `kit.music.score()` runs, in order (174 of 674 calls; the rest hit the cache): the arguments (`hex` has the exact floats), `noise_before` / `noise_after`, the `pp_bridge.voice` calls inside with the `n` of each `nz(n)`, the instrument's gain, the sample count and a SHA-256 of the samples |
| `sound/flute_vs_trombone/first_notes.npz` | The first three of those notes as arrays: `out` (the note), `voice` (before `fit` and gain) and, for the first, `gain_voice` |
| `sound/voices/FLUTE.npz`, `TROMBONE.npz` | `pp_bridge.voice(name, midi, vel, dur)` at 4 pitches x velocities 0.6, 0.9 x durations 0.12, 0.25, 1.0 at 48 kHz, stored as float32 to keep the set inside its size budget (the bar is -120 dBFS; `sha256` is of the float64 samples), each case with `noise` (state and inc before the call), `noise_after` and `draws`; two cases per file also have `raw44k` (float64), the note before resampling |
| `sound/voices/helpers.npz` | `inst_gain`, `note`, `fit`, `gliss`, `bend`, `vibrato`, `wah`, `colour`, `clank`, `hiss`, `roar`, `thunder`, `riser`, in the order they ran from the start state with an empty note cache. A shaping helper's `x` is the `out` member of an earlier `note` case. `colour` reads the fight, so its cases carry the two HP values it saw (`hp`) and `gl` = (gain, lead) |
| `sound/voices/scalars.json` | `h01`, `Thinner`, `h01_37`, `thin_37`, `q16` on fixed inputs |

Two things a port must copy from `note_calls.json`. The order of first-time notes is the order `score()` first asks
for them, bed and fighters interleaved as it calls them. And the first note of an instrument makes two voice calls:
its own, then the level-matching note of `kit.music.inst_gain` (`voice(name, (lo + hi) // 2, 0.8, 0.2)`), because
Python evaluates `fit(voice(...))` before `inst_gain(name)`. The FLUTE's gain therefore depends on where the noise
stream is when it is measured (`helpers.npz` measures it from the start state and gets a slightly different gain
than the episode does).

`voices.check()` proves the files from disk: it restores the start state, synthesises all 174 logged notes again in
order and compares each with its SHA-256 and recorded states (the first three with their arrays, bit for bit),
shows that the same note made from another state differs, and reproduces every voice vector from the state stored
with it. The export runs it before it lists anything in the manifest. `voices.py` also holds what `stems` and
`checks` share: the noise-state helpers and `NoteLog`.

### `stems`: the episode's soundtrack, bus by bus

For the stem parity tests of `bf-content` and the master of `bf-sound`. `kit.music.main()` for `flute_vs_trombone`
in its own interpreter; everything is in `studio/reference/sound/flute_vs_trombone/`.

| File | What |
| --- | --- |
| `audio.wav` | The mastered soundtrack exactly as the kit writes it: 16 bit, 48,000 Hz, stereo, 2,928,640 samples (61.013 s) |
| `stems.npz` | Every bus after the master (`master()`'s stems: the folded bus times its gain, the compressor's gain and the two limiter scales; no reverb), as `float32` `(96000, 2)` windows of 2.0 s from 0.0, 3.4, 12.0, 30.0, 43.0, T_KO - 1.0, T_KO + 3.0 and 59.0 s, plus `seam`, the last 0.25 s before the loop point. One case per window; its `arrays` map each bus to its samples `[i0, i0 + n)`. The sidecar's `master` holds the bus levels and sends, the order the buses are summed in, the reverb and compressor settings and the master's two scales `s1`, `s2` |
| `premaster.npz` | Every bus as `score()` leaves it (`mix.buses`, before the bus gains, the reverb, the fold and the master), the same windows, plus `tail`, the 0.25 s past the loop point that the master folds onto the start |
| `rms.json` | The RMS of every 100 ms block in dB (`20 * log10(sqrt(mean(x ** 2)) + 1e-12)` over both channels; the last block is shorter): `stems.db` for each stem and `master` (611 blocks), `premaster.db` for each bus (710 blocks: the whole 71 s buffer), `master_chain` (the reverb's wet part, the compressor's gain at the start of each block and its minimum inside it), and `sections`, the levels `kit.music.main()` prints |
| `master_case.npz` | A sparse synthetic mix on three buses (`drums`, `flute`, `fx`): the mono signals and where `Mix.place` puts them (`placements`), and `float64` windows of `kit.music.master()`'s result (`master` and each stem) at the start, the middle and the seam. Something crosses the loop point, so the fold is in it |

The brief's seven start times never hear the `glock` bus (it only plays in the back half's even bars), so the
window at 43.0 s was added; `stems.check()` fails if a bus is silent in every window. `silent` in each sidecar
lists the all-zero `[case, bus]` pairs. The windows are `float32` (the kit computes in `float64`): the comparison
bar is -120 dBFS and `float32` keeps 24 bits.

What the export proves before it lists anything: the noise generator starts at `pp_noise_start.json`; the notes it
logs equal `note_calls.json`; with the note cache emptied and the generator put back at the start state a second
`score()` gives every bus again bit for bit, and without putting it back the FLUTE's bus differs; `audio.wav` is the
16-bit form of the master it holds. The numbers inside the master come from a copy of `master()`'s lines in
`stems.py`, which must reproduce the kit's master and stems bit for bit or the export stops.

### `checks`: what the kit's checks say about the episode

For the check ports of `bf-export`. One file, `studio/reference/checks/flute_vs_trombone.json`:
`checks.<name>` for each of `focus`, `safezone`, `levels`, `band`, `loudness`, `flash` and `loop` has `pass` (the
kit's verdict; `loudness` has none, the kit only reports it), `printed` (the lines the kit prints, as they are),
`needs_frames` (how many drawn frames the check needs), `how` (what the numbers are), and the values behind the
printed lines as plain JSON numbers (the kit's `float64` values; times in seconds, levels in dB).

| Check | Drawn frames | Values |
| --- | --- | --- |
| `focus` | 0 (camera maths for all 1,830 frames) | `worst_coc`, `frames`, `points` |
| `safezone` | 244 (every 0.25 s: frame `int(round(t * 30))`) | `tracked`, `outside`, `overlaps`, and `boxes`: for each checked time every box `reelkit.ui` drew, `[x0, y0, x1, y1, kind, info]` |
| `levels` | 0 | `rows` (the 31 two-second windows: dBFS and hits/s), `ending`, `opening`, `build`, `spread`, `dips`, `targets`, and `module_output`, everything `python -m reelkit.levels` prints |
| `band` | 0 | per fighter its `band` and, for each of the five `spans`, `section` and `while_it_sounds` in dB over the bed; `bed_buses` |
| `loudness` | 0 | `intro_lufs`, `fight_lufs`, `intro_minus_fight` (what the kit returns), the sample `spans` |
| `flash` | 315 (frames 1230 to 1544 for this fight) | `t0`, `t1`, `f0`, `f1`, `general_flashes`, `flash_frames`, `worst_in_1s`, `largest_area` with its frame and time, and `area` for every frame |
| `loop` | 180 (the last 150 and the first 30) | `picture`: `seam`, `normal`, `ratio`, `steps` (the 178 frame-to-frame differences), `big_jumps`, `nearest_to_frame_0`; `audio`: the two 20 ms levels, their step, the typical sample step and the jump at the seam |

How the kit's checks are run without editing them: `focus`, `levels`, `band` and `loudness` are called as they are.
`flash` uses a `fork` pool, which Windows does not have, so the `multiprocessing` name inside `kit.checks` is swapped
for the call with a stand-in that hands the same frames' luminance from this module's `spawn` pool; `loop` gets its
frames the same way through `reelkit.loopcheck.frames_from_render`; `safezone` runs on a stand-in for the picture
module that replays the boxes `reelkit.ui` logged when the pool drew each frame. The verdicts and the printed lines
are the kit's own.

Several checks only print their numbers, rounded. For those the export computes the same formulas from the same
inputs and stops unless its values format to exactly the line the kit printed, so the unrounded values in the file
are the kit's numbers and not a second opinion. The soundtrack is built first, in the export's own interpreter and
cache folder, and must be byte for byte the committed `audio.wav`; `band` reads the 16-bit stem wavs that
`kit.music.main()` writes beside it.
