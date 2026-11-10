# BALLFIGHT Studio

A native Rust engine for BALLFIGHT episodes: the fight, the picture, the sound and the export, ported from the Python
kit in `projects/ballfight/` and checked against it. Design: `docs/studio/2026-11-10-ballfight-studio-design.md`.

## Crates

| Crate | Job | Depends on |
|---|---|---|
| `bf-core` | Shared types, the bar and beat grid, deterministic maths (`dmath`), the numpy-compatible random generator | nothing |
| `bf-sim` | The fight engine | core |
| `bf-story` | Story analysis, scoring, seed search, balance, filters | core, sim |
| `bf-picture` | The renderer | core |
| `bf-sound` | Synthesis and mixing | core |
| `bf-content` | Fighters, themes, the registry and the pair rules | sim, picture, sound |
| `bf-export` | Checks, encode, render job, output folders | story, picture, sound, content |
| `bf-cli` | The `ballfight` command line | all |

## Building and testing

Run everything from this folder (`studio/`). The toolchain is pinned in `rust-toolchain.toml`.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Determinism

Results must be bit-identical on every PC. The engine crates use `f64` in the Python operation order, never fused
multiply-add, and call every transcendental through `bf_core::dmath`; `crates/bf-core/tests/no_std_maths.rs` scans
the sources and fails on any direct call such as `x.sin()` or `x.mul_add(a, b)`. No result may depend on hash-map
order or on the number of threads.

## Reference data

`reference/` holds the vectors exported from the Python kit by `tools/studio_export/`, each listed in
`reference/manifest.json` with its SHA-256.
