//! Playback maths. Positions are samples of the 48 kHz soundtrack, which is the clock (§7): the frame on screen is
//! whichever frame the sound has reached. All of it is integer arithmetic, so no rounding can put a position on the
//! wrong side of a frame boundary.

use crate::model::{FPS, NA, NF, SAMPLES_PER_FRAME};

/// The last frame of a video.
const LAST: u32 = NF - 1;

/// The length of a video as the transport shows it.
pub const TOTAL: &str = "1:01.00";

/// The frame showing at sample `pos`: `min(pos / 1600, 1829)`. The soundtrack runs 640 samples past the end of the
/// last frame (2,928,640 samples against 1,830 x 1,600 = 2,928,000); those samples, like any position past the end,
/// show the last frame.
pub fn frame_of(pos: u64) -> u32 {
    (pos / SAMPLES_PER_FRAME).min(u64::from(LAST)) as u32
}

/// The first sample of `frame`: `frame * 1600`.
pub fn pos_of(frame: u32) -> u64 {
    u64::from(frame) * SAMPLES_PER_FRAME
}

/// Moves the position `by` samples. Returns the new position and whether playback goes on. When `looping`, the
/// position wraps at `NA` (the loop point: the last sample flows into the first) and playback always goes on;
/// otherwise a move that reaches or passes `NA` stops on `NA - 1`, the last sample. A paused player moves by 0.
pub fn advance(pos: u64, by: u64, looping: bool) -> (u64, bool) {
    if looping {
        // Both reduced before adding, so no position or move can overflow.
        ((pos % NA + by % NA) % NA, true)
    } else {
        let next = pos.saturating_add(by);
        if next >= NA { (NA - 1, false) } else { (next, true) }
    }
}

/// The frame `by` frames from `frame`, kept in `0..=1829`.
pub fn step(frame: u32, by: i32) -> u32 {
    (i64::from(frame) + i64::from(by)).clamp(0, i64::from(LAST)) as u32
}

/// The time of `frame` as "m:ss.hh": whole hundredths of a second, `frame * 100 / 30` rounded down.
pub fn timecode(frame: u32) -> String {
    let hundredths = u64::from(frame) * 100 / u64::from(FPS);
    format!(
        "{}:{:02}.{:02}",
        hundredths / 6_000,
        hundredths / 100 % 60,
        hundredths % 100
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_of_divides_by_1600_and_stops_on_the_last_frame() {
        assert_eq!(frame_of(0), 0);
        assert_eq!(frame_of(1_599), 0);
        assert_eq!(frame_of(1_600), 1);
        assert_eq!(frame_of(2_927_999), 1_829);
        // The 640 samples after the last frame's 1,600, up to the last sample, show the last frame.
        assert_eq!(frame_of(2_928_000), 1_829);
        assert_eq!(frame_of(2_928_639), 1_829);
    }

    #[test]
    fn positions_past_the_end_show_the_last_frame() {
        assert_eq!(frame_of(NA), 1_829);
        assert_eq!(frame_of(NA + 5_000), 1_829);
        assert_eq!(frame_of(u64::MAX), 1_829);
    }

    #[test]
    fn a_scrub_between_frames_shows_the_frame_it_is_inside() {
        assert_eq!(frame_of(pos_of(942) + 800), 942);
        assert_eq!(frame_of(pos_of(942) + 1_599), 942);
        assert_eq!(frame_of(pos_of(942) - 1), 941);
    }

    #[test]
    fn pos_of_is_the_frames_first_sample() {
        assert_eq!(pos_of(0), 0);
        assert_eq!(pos_of(942), 1_507_200);
        assert_eq!(pos_of(1_829), 2_926_400);
        for frame in 0..NF {
            assert_eq!(frame_of(pos_of(frame)), frame);
        }
    }

    #[test]
    fn advance_wraps_at_the_loop_point_when_looping() {
        assert_eq!(advance(2_928_000, 1_000, true), (360, true));
        assert_eq!(advance(NA - 1, 1, true), (0, true));
        assert_eq!(advance(NA - 2, 1, true), (NA - 1, true));
        assert_eq!(advance(0, 1_600, true), (1_600, true));
        assert_eq!(advance(NA + 10, 0, true), (10, true));
        assert_eq!(advance(u64::MAX, u64::MAX, true), ((u64::MAX % NA) * 2 % NA, true));
    }

    #[test]
    fn advance_stops_on_the_last_sample_when_not_looping() {
        assert_eq!(advance(2_928_000, 1_000, false), (2_928_639, false));
        assert_eq!(advance(NA - 2, 1, false), (NA - 1, true));
        assert_eq!(advance(NA - 1, 1, false), (NA - 1, false));
        assert_eq!(advance(NA + 5_000, 0, false), (NA - 1, false));
        assert_eq!(advance(u64::MAX, u64::MAX, false), (NA - 1, false));
        assert_eq!(advance(0, 1_600, false), (1_600, true));
    }

    #[test]
    fn a_paused_player_stays_where_it_is() {
        assert_eq!(advance(0, 0, false), (0, true));
        assert_eq!(advance(1_507_200, 0, false), (1_507_200, true));
        assert_eq!(advance(1_507_200, 0, true), (1_507_200, true));
        assert_eq!(advance(NA - 1, 0, true), (NA - 1, true));
    }

    #[test]
    fn step_keeps_the_frame_inside_the_video() {
        assert_eq!(step(0, -1), 0);
        assert_eq!(step(1_829, 1), 1_829);
        assert_eq!(step(10, -3), 7);
        assert_eq!(step(10, 3), 13);
        assert_eq!(step(0, i32::MIN), 0);
        assert_eq!(step(0, i32::MAX), 1_829);
        assert_eq!(step(u32::MAX, 0), 1_829);
    }

    #[test]
    fn timecode_is_whole_hundredths() {
        assert_eq!(timecode(0), "0:00.00");
        assert_eq!(timecode(942), "0:31.40");
        assert_eq!(timecode(1), "0:00.03");
        assert_eq!(timecode(2), "0:00.06");
        assert_eq!(timecode(3), "0:00.10");
        assert_eq!(timecode(1_799), "0:59.96");
        assert_eq!(timecode(1_800), "1:00.00");
        assert_eq!(timecode(1_829), "1:00.96");
        assert_eq!(TOTAL, "1:01.00");
    }
}
