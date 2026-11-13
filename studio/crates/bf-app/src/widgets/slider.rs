//! The slider (interface reference, section 3): a 2 px `LINE` track, a `MUTED` fill from the left to the knob and a
//! 3 x 14 `TEXT_STRONG` bar as the knob, in a row 26 tall. Never coloured.
//!
//! It is painted by hand. It takes drags and clicks anywhere on its row; with the keyboard on it the left and right
//! arrows move it one step (a screen reader's increase and decrease do the same). It keeps 4 px free inside its row
//! on every side, and its focus ring (2 px `TEXT_STRONG`, 2 px outside the track and knob) is painted there, so a
//! panel's clip rectangle never cuts the ring off.
//!
//! Whatever it is handed, it never produces a value that is not a finite number inside the range:
//! - a range whose ends are the wrong way round is used the right way round;
//! - an end that is not a finite number is replaced by the other end (so the range is empty), and a range with no
//!   finite end at all is `(0, 0)`;
//! - an empty range gives its one value;
//! - a step that is not a finite number above zero (zero, negative, not a number, infinite) snaps nothing: the value
//!   is only kept inside the range, and an arrow key moves it a hundredth of the range;
//! - a value that is not a number is shown as the range's low end, an infinite one as the nearer end. The caller's
//!   value is left alone until the person moves the slider.

use egui::accesskit::Action;
use egui::{EventFilter, Key, Modifiers, Rect, Sense, Ui, WidgetInfo, pos2, vec2};

use super::buttons::focus_ring;
use crate::theme::{CONTROL_H, FAINT, LINE, MUTED, TEXT_STRONG};

/// The track's thickness.
const TRACK_H: f32 = 2.0;
/// The knob.
const KNOB_W: f32 = 3.0;
const KNOB_H: f32 = 14.0;
/// Room kept free inside the row on every side for the focus ring (2 px of gap, then 2 px of ring).
const RING_ROOM: f32 = 4.0;
/// The narrowest slider: the ring's room on both sides, the knob and one point to move in.
const MIN_W: f32 = 2.0 * RING_ROOM + KNOB_W + 1.0;
/// Without a usable step, one arrow press moves this share of the range.
const KEY_SHARE: f64 = 0.01;

/// What a slider did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SliderOut {
    /// The person changed the value this frame (with the pointer, the keyboard or a screen reader).
    pub changed: bool,
    /// A drag or a click ended this frame, an arrow key that moved the slider went up (once per hold, however
    /// often it repeated), or a screen reader stepped it: the moment to act on the new value.
    pub released: bool,
}

/// `range` made safe: both ends finite and the low end first (see the module's notes).
pub(crate) fn tidy_range(range: (f64, f64)) -> (f64, f64) {
    let (a, b) = match (range.0.is_finite(), range.1.is_finite()) {
        (true, true) => range,
        (true, false) => (range.0, range.0),
        (false, true) => (range.1, range.1),
        (false, false) => (0.0, 0.0),
    };
    if a <= b { (a, b) } else { (b, a) }
}

/// `value` inside the tidy range `(lo, hi)`: not a number gives `lo`, an infinity the nearer end.
pub(crate) fn clamp(value: f64, (lo, hi): (f64, f64)) -> f64 {
    if value.is_nan() { lo } else { value.clamp(lo, hi) }
}

/// Half the distance from `a` up to `b`, which never overflows for finite `a` and `b`.
pub(crate) fn half_span(a: f64, b: f64) -> f64 {
    b / 2.0 - a / 2.0
}

/// The step, if it is a finite number above zero.
fn tidy_step(step: f64) -> Option<f64> {
    (step.is_finite() && step > 0.0).then_some(step)
}

/// `value` kept inside the tidy `range` and moved to the nearest of the steps counted from its low end, or to its
/// high end when that is nearer (so both ends can always be reached).
fn snap(value: f64, range: (f64, f64), step: Option<f64>) -> f64 {
    let (lo, hi) = range;
    let value = clamp(value, range);
    let Some(step) = step else { return value };
    let steps = (half_span(lo, value) / (step / 2.0)).round();
    let snapped = lo + steps * step;
    if !snapped.is_finite() {
        return value;
    }
    let nearest = if (hi - value).abs() < (value - snapped).abs() {
        hi
    } else {
        snapped
    };
    clamp(nearest, range)
}

/// Where `value` lies in `range`, from 0 (low end) to 1 (high end); 0 for an empty range.
fn share(value: f64, range: (f64, f64)) -> f64 {
    let (lo, hi) = tidy_range(range);
    let span = half_span(lo, hi);
    if span <= 0.0 {
        return 0.0;
    }
    let t = half_span(lo, clamp(value, (lo, hi))) / span;
    if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 }
}

/// The x of the knob's centre for `value` on a travel that starts at `left` and is `width` long. It is always
/// between `left` and `left + width`; a width that is not a finite number above zero puts it at `left`, and a `left`
/// that is not finite gives 0.
pub fn knob_x(value: f64, range: (f64, f64), left: f32, width: f32) -> f32 {
    if !left.is_finite() {
        return 0.0;
    }
    let width = if width.is_finite() && width > 0.0 { width } else { 0.0 };
    left + width * share(value, range) as f32
}

/// The value at `x` on a travel that starts at `left` and is `width` long: kept inside the range and snapped to
/// `step` (counted from the range's low end; the high end is also reachable). Left of the travel gives the low end
/// exactly, right of it the high end exactly; an `x`, `left` or `width` that cannot be used gives the low end.
pub fn value_at(x: f32, range: (f64, f64), step: f64, left: f32, width: f32) -> f64 {
    let (lo, hi) = tidy_range(range);
    let usable = x.is_finite() && left.is_finite() && width.is_finite() && width > 0.0;
    if !usable {
        return lo;
    }
    let t = (f64::from(x) - f64::from(left)) / f64::from(width);
    if t <= 0.0 {
        return lo;
    }
    if t >= 1.0 {
        return hi;
    }
    snap(lo * (1.0 - t) + hi * t, (lo, hi), tidy_step(step))
}

/// A slider named `name` for `value` in `range`, snapped to `step`, `width` wide (at least 12) and 26 tall. See the
/// module's notes for what it does with values, ranges and steps it cannot use. Inside a disabled parent its fill
/// and knob are `FAINT` and it takes no input.
pub fn bar_slider(ui: &mut Ui, name: &str, value: &mut f64, range: (f64, f64), step: f64, width: f32) -> SliderOut {
    let range = tidy_range(range);
    let step = tidy_step(step);
    let width = if width.is_finite() { width.max(MIN_W) } else { MIN_W };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_H), Sense::click_and_drag());
    let enabled = ui.is_enabled();
    let control = rect.shrink(RING_ROOM);
    let travel_left = control.min.x + KNOB_W / 2.0;
    let travel = control.width() - KNOB_W;

    let mut next = clamp(*value, range);
    let mut acted = false;
    let mut out = SliderOut::default();
    if enabled {
        if response.is_pointer_button_down_on()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            next = value_at(pointer.x, range, step.unwrap_or(0.0), travel_left, travel);
            acted = true;
        }
        let reader = ui.input(|input| {
            input.num_accesskit_action_requests(response.id, Action::Increment) as f64
                - input.num_accesskit_action_requests(response.id, Action::Decrement) as f64
        });
        let mut keys = 0.0;
        if response.has_focus() {
            // The arrows belong to the slider while it has the keyboard: without this egui would also move the
            // focus to the neighbouring control on the same press.
            ui.memory_mut(|memory| {
                let filter = EventFilter {
                    horizontal_arrows: true,
                    ..EventFilter::default()
                };
                memory.set_focus_lock_filter(response.id, filter);
            });
            keys = ui.input_mut(|input| {
                input.count_and_consume_key(Modifiers::NONE, Key::ArrowRight) as f64
                    - input.count_and_consume_key(Modifiers::NONE, Key::ArrowLeft) as f64
            });
        }
        if reader + keys != 0.0 {
            let unit = step.unwrap_or(2.0 * half_span(range.0, range.1) * KEY_SHARE);
            next = snap(next + (reader + keys) * unit, range, step);
            acted = true;
        }
        // A screen reader's step is finished at once. An arrow key is finished when it goes up: while it is held
        // and repeats, every repeat moves the slider but only the key going up releases it, once per hold.
        let held_id = response.id.with("arrow held");
        let mut held = ui.data(|data| data.get_temp::<bool>(held_id)).unwrap_or(false);
        if keys != 0.0 {
            held = true;
        }
        let arrow_down = ui.input(|input| input.key_down(Key::ArrowLeft) || input.key_down(Key::ArrowRight));
        if held && !arrow_down {
            held = false;
            out.released = true;
        }
        ui.data_mut(|data| data.insert_temp(held_id, held));
        out.released |= reader != 0.0 || response.drag_stopped() || response.clicked();
    }
    // `next` is always finite and inside the range; a value that is not a number differs from it, so the first move
    // writes a usable one.
    if acted && next != *value {
        *value = next;
        out.changed = true;
    }

    response.widget_info(|| WidgetInfo::slider(enabled, next, name));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_min_numeric_value(range.0);
        node.set_max_numeric_value(range.1);
        if let Some(step) = step {
            node.set_numeric_value_step(step);
        }
        if enabled {
            if next < range.1 {
                node.add_action(Action::Increment);
            }
            if next > range.0 {
                node.add_action(Action::Decrement);
            }
        }
    });

    if ui.is_rect_visible(rect) {
        // A disabled slider is drawn FAINT here, at full strength, not faded by egui.
        let mut painter = ui.painter().clone();
        if !enabled {
            painter.set_opacity(1.0);
        }
        let painter = &painter;
        let middle = control.center().y;
        let track_top = (middle - TRACK_H / 2.0).round();
        let track = Rect::from_min_max(pos2(control.min.x, track_top), pos2(control.max.x, track_top + TRACK_H));
        painter.rect_filled(track, 0.0, LINE);
        let knob_left = (knob_x(next, range, travel_left, travel) - KNOB_W / 2.0).round();
        let (fill, knob) = if enabled { (MUTED, TEXT_STRONG) } else { (FAINT, FAINT) };
        if knob_left > control.min.x {
            let filled = Rect::from_min_max(track.min, pos2(knob_left, track.max.y));
            painter.rect_filled(filled, 0.0, fill);
        }
        let knob_top = (middle - KNOB_H / 2.0).round();
        let knob_rect = Rect::from_min_size(pos2(knob_left, knob_top), vec2(KNOB_W, KNOB_H));
        painter.rect_filled(knob_rect, 0.0, knob);
        if response.has_focus() {
            focus_ring(painter, control);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGE: (f64, f64) = (0.05, 3.0);

    #[test]
    fn the_knob_sits_where_the_value_is() {
        let x = knob_x(2.0, RANGE, 100.0, 200.0);
        assert!((x - (100.0 + 200.0 * (1.95 / 2.95))).abs() < 0.01, "{x}");
        assert_eq!(knob_x(0.05, RANGE, 100.0, 200.0), 100.0);
        assert_eq!(knob_x(3.0, RANGE, 100.0, 200.0), 300.0);
    }

    #[test]
    fn a_value_is_read_from_a_position_and_snapped() {
        let value = value_at(100.0 + 0.66 * 200.0, RANGE, 0.01, 100.0, 200.0);
        assert!((value - 2.0).abs() < 1e-9, "{value}");
        assert_eq!(value_at(-50.0, RANGE, 0.01, 100.0, 200.0), 0.05);
        assert_eq!(value_at(9999.0, RANGE, 0.01, 100.0, 200.0), 3.0);
    }

    #[test]
    fn a_whole_step_gives_whole_numbers() {
        for i in 0..=400 {
            let x = 100.0 + i as f32 * 0.5;
            let value = value_at(x, (24.0, 42.0), 1.0, 100.0, 200.0);
            assert_eq!(value, value.round(), "x {x} gives {value}");
            assert!((24.0..=42.0).contains(&value));
        }
    }

    #[test]
    fn both_ends_are_reachable_when_the_step_does_not_divide_the_range() {
        assert_eq!(value_at(299.0, (0.0, 1.0), 0.3, 100.0, 200.0), 1.0);
        assert_eq!(value_at(180.0, (0.0, 1.0), 0.3, 100.0, 200.0), 0.3);
        assert_eq!(snap(0.98, (0.0, 1.0), Some(0.3)), 1.0);
    }

    /// Ranges, steps, positions and values nobody should pass: none gives a value outside the range or a knob off
    /// its travel.
    #[test]
    fn unusable_inputs_never_give_a_value_outside_the_range() {
        let ranges = [
            (0.05, 3.0),
            (3.0, 0.05),
            (1.0, 1.0),
            (f64::NAN, 2.0),
            (2.0, f64::INFINITY),
            (f64::NEG_INFINITY, f64::INFINITY),
            (f64::NAN, f64::NAN),
            (-f64::MAX, f64::MAX),
        ];
        let steps = [0.01, 0.0, -1.0, f64::NAN, f64::INFINITY, 1e-300, 1e300];
        let xs = [
            -1e30,
            0.0,
            100.0,
            150.0,
            300.0,
            1e30,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ];
        let values = [0.0, 1.5, -7.0, 1e300, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        let widths = [200.0, 0.0, -5.0, f32::NAN, f32::INFINITY];
        for range in ranges {
            let (lo, hi) = tidy_range(range);
            assert!(lo.is_finite() && hi.is_finite() && lo <= hi, "{range:?}");
            for step in steps {
                for x in xs {
                    for width in widths {
                        let value = value_at(x, range, step, 100.0, width);
                        let fine = value.is_finite() && (lo..=hi).contains(&value);
                        assert!(fine, "{range:?} {step} {x} {width}: {value}");
                    }
                }
            }
            for value in values {
                for width in [200.0, 0.0, -5.0, f32::NAN] {
                    let x = knob_x(value, range, 100.0, width);
                    assert!(
                        x.is_finite() && (100.0..=300.0).contains(&x),
                        "{range:?} {value} {width}: {x}"
                    );
                }
                for step in steps {
                    let snapped = snap(value, (lo, hi), tidy_step(step));
                    let fine = snapped.is_finite() && (lo..=hi).contains(&snapped);
                    assert!(fine, "{range:?} {value} {step}: {snapped}");
                }
            }
        }
    }

    #[test]
    fn what_each_unusable_input_means() {
        assert_eq!(tidy_range((3.0, 0.05)), (0.05, 3.0), "a reversed range is turned round");
        assert_eq!(tidy_range((f64::NAN, 2.0)), (2.0, 2.0), "a missing end takes the other");
        assert_eq!(
            tidy_range((2.0, f64::INFINITY)),
            (2.0, 2.0),
            "an infinite end takes the other"
        );
        assert_eq!(
            tidy_range((f64::NAN, f64::INFINITY)),
            (0.0, 0.0),
            "no finite end at all"
        );
        assert_eq!(
            value_at(250.0, (1.0, 1.0), 0.1, 100.0, 200.0),
            1.0,
            "an empty range gives its value"
        );
        assert_eq!(
            knob_x(1.0, (1.0, 1.0), 100.0, 200.0),
            100.0,
            "and puts the knob at the left"
        );
        assert_eq!(
            value_at(150.0, (0.0, 1.0), 0.0, 100.0, 200.0),
            0.25,
            "a zero step snaps nothing"
        );
        assert_eq!(
            value_at(150.0, (0.0, 1.0), -0.3, 100.0, 200.0),
            0.25,
            "nor does a negative one"
        );
        assert_eq!(
            value_at(150.0, (0.0, 1.0), f64::NAN, 100.0, 200.0),
            0.25,
            "nor one that is not a number"
        );
        assert_eq!(
            value_at(150.0, (2.0, 0.0), 0.5, 100.0, 200.0),
            0.5,
            "a reversed range reads the right way"
        );
        assert_eq!(
            value_at(f32::NAN, (0.0, 1.0), 0.1, 100.0, 200.0),
            0.0,
            "no position gives the low end"
        );
        assert_eq!(
            value_at(150.0, (0.0, 1.0), 0.1, 100.0, 0.0),
            0.0,
            "no width gives the low end"
        );
        assert_eq!(
            knob_x(f64::NAN, (0.0, 1.0), 100.0, 200.0),
            100.0,
            "not a number sits at the low end"
        );
        assert_eq!(
            knob_x(f64::INFINITY, (0.0, 1.0), 100.0, 200.0),
            300.0,
            "an infinity at the nearer end"
        );
        assert_eq!(
            knob_x(0.5, (0.0, 1.0), 100.0, -1.0),
            100.0,
            "no width puts the knob at the left"
        );
        assert_eq!(
            knob_x(0.5, (0.0, 1.0), f32::NAN, 200.0),
            0.0,
            "a left that is not a number"
        );
        let middle = value_at(200.0, (-f64::MAX, f64::MAX), 0.0, 100.0, 200.0);
        assert_eq!(middle, 0.0, "the widest range does not overflow");
    }
}
