//! Reader for the Studio's reference vectors (`studio/reference`), shared by every parity test.
//!
//! The files follow `vectors-convention`: `.npz` archives of plain numeric `.npy` members, each with a JSON
//! sidecar that lists the cases; numbers that must match bit for bit are Python `float.hex()` strings. This is a
//! dev-support crate (engine crates use it only under `[dev-dependencies]`), so a malformed reference file is a
//! panic with a message that names the file: a broken reference set must fail tests loudly.

pub mod diff;
pub mod hexf;
pub mod npz;
pub mod paths;
pub mod png;
pub mod sidecar;

pub use self::diff::{ImgDiff, db_max_diff, diff_u8, max_rel_diff};
pub use self::hexf::{hex_arg, parse_hex_f64};
pub use self::npz::{NpArray, NpData, Npz, read_npy};
pub use self::paths::{reference, reference_dir, repo_root};
pub use self::png::{read_png, write_png_rgb};
pub use self::sidecar::{Case, Sidecar, Vectors};
