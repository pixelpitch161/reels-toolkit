//! The checks, the ffmpeg encode, the render job and the output folders.

pub mod checks;

pub use self::checks::{BandPlan, CheckResult, pre_render};
