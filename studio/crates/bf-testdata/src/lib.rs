//! Reader for the Studio's reference vectors (`studio/reference`), shared by every parity test.
//!
//! The files follow `vectors-convention`: `.npz` archives of plain numeric `.npy` members, each with a JSON
//! sidecar that lists the cases; numbers that must match bit for bit are Python `float.hex()` strings. This is a
//! dev-support crate (engine crates use it only under `[dev-dependencies]`), so a malformed reference file is a
//! panic with a message that names the file: a broken reference set must fail tests loudly.
//!
//! Milestone 3 adds the storage layout: the manifest sharded per fighter, theme and pair folder (`manifest`), the
//! pinned cache of regenerated files under `studio/reference/.cache/m3/` (`cache`), and the size budgets
//! (`budget`).

pub mod budget;
pub mod cache;
pub mod diff;
pub mod hexf;
pub mod manifest;
pub mod npz;
pub mod paths;
pub mod png;
pub mod sidecar;

pub use self::budget::{Budget, TRIPWIRE, budget_report, budget_rows, reference_files, tripwire_warning};
pub use self::cache::{Pin, cache_bytes, cache_command, cache_file, cache_root, export_command, pins};
pub use self::diff::{ImgDiff, db_max_diff, diff_u8, max_rel_diff};
pub use self::hexf::{hex_arg, parse_hex_f64};
pub use self::manifest::{ManifestEntry, manifest_entries, manifest_shards, shard_of};
pub use self::npz::{NpArray, NpData, Npz, read_npy};
pub use self::paths::{reference, reference_dir, repo_root};
pub use self::png::{read_png, write_png_rgb};
pub use self::sidecar::{Case, Sidecar, Vectors};
