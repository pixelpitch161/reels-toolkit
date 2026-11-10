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

An export refuses to run unless `PYTHONHASHSEED` is `0` and numpy, scipy and pillow are exactly the versions in
`requirements.txt`.

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

`manifest.add(path)` updates it under a lock file and replaces it in one step, so exports may run at the same time.
When an export reproduces a file bit for bit with the same tools, its entry is left as it is (only `commit` would
change), so re-running the export on an unchanged kit leaves the working tree clean. Files an export writes outside
`studio/reference/` are listed too, by their repo-relative path.

## Adding an export

Add a module to this folder. Nothing else changes: the entry point scans the package.

```python
from . import common, manifest

OUT = common.ref('thing', 'data.json')          # studio/reference/thing/data.json


def run():
    common.write_json(OUT, {'format': 1, 'values': [common.hexf(0.1)]})
    manifest.add(OUT)


EXPORTS = {'thing': run}                         # python -m tools.studio_export thing
# ORDER = {'thing': 60}                          # only if it must run after the default 50s under `all`
```

## What each export writes

| Name | Module | Files |
| --- | --- | --- |
| `py` | `py_vectors.py` | `studio/reference/py/vectors.json` |
| `pyhash` | `pyhash_vectors.py` | `studio/reference/pyhash/vectors.json` |
| `rng` | `rng_vectors.py` | `studio/reference/rng/vectors.json` |

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
