//! The lead ribbon (interface reference, section 4): who leads, over the 61 s of the video. A 1 px `LINE` centre
//! line; on the timeline the K.O. window, 49 to 53 s, shaded `RAISED` over the full height; the left fighter's colour
//! filled between the centre line and the lead curve above the line while it leads, the right fighter's colour below
//! while it leads; a 2 px `TEXT_STRONG` bar at the K.O., inset 2 from top and bottom. The curve starts on the centre
//! line at FIGHT!, has one point per hit, straight lines between points, and returns to the line at the K.O. A 42 HP
//! lead fills a side, leaving 1 px.
//!
//! It is 41 tall on the timeline (drawn there with [`paint`], where the timeline's scrub owns the input) and 28 tall
//! in a shortlist row ([`small`], which takes no input and so needs no focus ring or name of its own).
//!
//! It never panics, never divides by zero and never paints outside its rect, whatever story it is handed:
//! - a hit whose time or lead is not a finite number is left out, and so is a hit before FIGHT! or after the K.O.
//!   (after 61 s when there is no K.O.); the hits left are put in time order;
//! - a FIGHT! time that is not a number counts as 0, and one outside 0 to 61 s is moved to the nearer end;
//! - a K.O. time that is not a finite number, or lies before FIGHT! or after 61 s, counts as no K.O.: no bar, and the
//!   curve ends at the last hit;
//! - a lead beyond 42 HP fills its side and no more; hits on one instant draw a vertical edge;
//! - an empty lead draws the centre line only (and the bar, if there is a K.O.); a curve of one point draws no fill;
//! - the ability marks are not the ribbon's business (the timeline's lanes draw them), so they change nothing here;
//! - a rect that is not finite or has no width or no height draws nothing; one too low for a fill draws the line;
//! - in [`small`], a width that is not a finite number above zero takes no room and draws nothing.
//!
//! Inside a disabled parent the small ribbon draws both fills and the bar `FAINT` at full strength (not faded by
//! egui), and still takes no input.

use egui::{Color32, Mesh, Painter, Rect, Response, Sense, Shape, Ui, pos2, vec2};

use crate::model::story::StoryView;
use crate::theme::{FAINT, LINE, RAISED, TEXT_STRONG, rgb};

/// The video's length in seconds: the ribbon's time runs from 0 to this, left to right.
pub const DUR: f32 = 61.0;
/// The K.O. window, in seconds.
const KO_WINDOW: (f32, f32) = (49.0, 53.0);
/// The lead, in HP, that fills a side.
const FULL_LEAD: f32 = 42.0;
/// The small ribbon's height.
const SMALL_H: f32 = 28.0;
/// The K.O. bar's width, and how far it stops short of the top and the bottom.
const BAR_W: f32 = 2.0;
const BAR_INSET: f32 = 2.0;

/// The x of time `t` on a ribbon that starts at `left` and is `width` wide: `left + t / 61 * width`.
pub fn x_of(t: f32, left: f32, width: f32) -> f32 {
    left + t / DUR * width
}

/// The time at `x` on a ribbon that starts at `left` and is `width` wide, kept inside 0 to 61 s. An `x` that is
/// not a number, or a `left` or `width` that cannot be used (not finite, or a width not above zero), gives 0.
pub fn t_of(x: f32, left: f32, width: f32) -> f32 {
    let usable = !x.is_nan() && left.is_finite() && width.is_finite() && width > 0.0;
    if !usable {
        return 0.0;
    }
    let t = (x - left) / width * DUR;
    if t.is_nan() { 0.0 } else { t.clamp(0.0, DUR) }
}

/// How far a lead of `lead` HP (either fighter's) reaches from the centre line of a ribbon whose half height is
/// `mid`: `min(|lead| * (mid - 1) / 42, mid - 1)`. Never negative: a lead that is not a number, or a ribbon with no
/// room (`mid` not finite or 1 and under), gives 0.
pub fn lead_height(lead: f32, mid: f32) -> f32 {
    let cap = mid - 1.0;
    if lead.is_nan() || !cap.is_finite() || cap <= 0.0 {
        return 0.0;
    }
    (lead.abs() * cap / FULL_LEAD).min(cap)
}

/// FIGHT!, inside the video.
fn fight_time(story: &StoryView) -> f32 {
    let t = story.t_fight as f32;
    if t.is_nan() { 0.0 } else { t.clamp(0.0, DUR) }
}

/// The K.O., when there is one that can be drawn: finite, not before FIGHT! and not after the video's end.
fn ko_time(story: &StoryView) -> Option<f32> {
    let t = story.t_ko? as f32;
    (t.is_finite() && t >= fight_time(story) && t <= DUR).then_some(t)
}

/// The lead curve as `(t, lead)` points: `(t_fight, 0)`, then one point per hit in time order, then `(t_ko, 0)`
/// when there is a K.O. Hits that cannot be used are left out (see the module's notes).
pub fn curve(story: &StoryView) -> Vec<(f32, f32)> {
    let start = fight_time(story);
    let ko = ko_time(story);
    let end = ko.unwrap_or(DUR);
    let mut hits: Vec<(f32, f32)> = story
        .lead
        .iter()
        .copied()
        .filter(|&(t, lead)| t.is_finite() && lead.is_finite() && t >= start && t <= end)
        .collect();
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut points = Vec::with_capacity(hits.len() + 2);
    points.push((start, 0.0));
    points.extend(hits);
    if let Some(ko) = ko {
        points.push((ko, 0.0));
    }
    points
}

/// Whether a ribbon can be drawn in `rect`: finite, with some width and some height.
fn usable(rect: Rect) -> bool {
    rect.is_finite() && rect.width() > 0.0 && rect.height() > 0.0
}

/// Adds one quad from `left` to `right`, between the centre line and the heights `y_left` and `y_right`.
fn quad(mesh: &mut Mesh, (left, right): (f32, f32), (y_left, y_right): (f32, f32), centre: f32, colour: Color32) {
    let base = mesh.vertices.len() as u32;
    mesh.colored_vertex(pos2(left, centre), colour);
    mesh.colored_vertex(pos2(left, y_left), colour);
    mesh.colored_vertex(pos2(right, y_right), colour);
    mesh.colored_vertex(pos2(right, centre), colour);
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
}

/// The fills of `story` in `rect`: for each pair of neighbouring curve points one quad above the centre line in
/// `colors[0]` (where the left fighter leads) and one below it in `colors[1]` (where the right one does). Where the
/// lead changes hands between two points, each side's quad meets the centre line where the straight line between
/// them crosses it, so the fills follow the curve exactly. Empty when `rect` cannot be used or the curve has one
/// point; every vertex lies inside `rect`.
pub fn mesh(story: &StoryView, rect: Rect, colors: [Color32; 2]) -> Mesh {
    let mut mesh = Mesh::default();
    if !usable(rect) {
        return mesh;
    }
    let points = curve(story);
    let centre = rect.center().y;
    let mid = rect.height() / 2.0;
    let x = |t: f32| x_of(t, rect.min.x, rect.width()).clamp(rect.min.x, rect.max.x);
    // The height above the line (positive) or below it (negative).
    let signed = |lead: f32| {
        let height = lead_height(lead, mid);
        if lead < 0.0 { -height } else { height }
    };
    for pair in points.windows(2) {
        let ((t0, lead0), (t1, lead1)) = (pair[0], pair[1]);
        let (x0, x1) = (x(t0), x(t1));
        let (s0, s1) = (signed(lead0), signed(lead1));
        let down = s0 > 0.0 && s1 < 0.0;
        let up = s0 < 0.0 && s1 > 0.0;
        // Where the straight line crosses the centre: `s0 - s1` is not zero when the two have opposite signs.
        let cross = if down || up {
            x0 + (x1 - x0) * (s0 / (s0 - s1))
        } else {
            x1
        };
        let above = (if up { cross } else { x0 }, if down { cross } else { x1 });
        let heights = (centre - s0.max(0.0), centre - s1.max(0.0));
        quad(&mut mesh, above, heights, centre, colors[0]);
        let below = (if down { cross } else { x0 }, if up { cross } else { x1 });
        let heights = (centre + (-s0).max(0.0), centre + (-s1).max(0.0));
        quad(&mut mesh, below, heights, centre, colors[1]);
    }
    mesh
}

/// Draws the ribbon of `story` in `rect` with `painter`, in this order: the K.O. window when `ko_window` (the
/// timeline's), the centre line, the fills in the two fighters' `colors` (left, right), and the K.O. bar. Nothing
/// is drawn outside `rect`, and nothing at all when `rect` cannot be used.
pub fn paint(painter: &Painter, rect: Rect, story: &StoryView, colors: [[u8; 3]; 2], ko_window: bool) {
    paint_in(
        painter,
        rect,
        story,
        [rgb(colors[0]), rgb(colors[1])],
        TEXT_STRONG,
        ko_window,
    );
}

/// [`paint`] with the fill colours and the bar's colour given.
fn paint_in(painter: &Painter, rect: Rect, story: &StoryView, fills: [Color32; 2], bar: Color32, ko_window: bool) {
    if !usable(rect) {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    let x = |t: f32| x_of(t, rect.min.x, rect.width()).round();
    if ko_window {
        let window = Rect::from_x_y_ranges(x(KO_WINDOW.0)..=x(KO_WINDOW.1), rect.y_range());
        painter.rect_filled(window.intersect(rect), 0.0, RAISED);
    }
    let line_top = (rect.center().y - 0.5).round();
    let line = Rect::from_x_y_ranges(rect.x_range(), line_top..=line_top + 1.0);
    painter.rect_filled(line.intersect(rect), 0.0, LINE);
    let fill = mesh(story, rect, fills);
    if !fill.is_empty() {
        painter.add(Shape::mesh(fill));
    }
    let (top, bottom) = (rect.min.y + BAR_INSET, rect.max.y - BAR_INSET);
    if let Some(ko) = ko_time(story)
        && bottom > top
    {
        // Kept inside the ribbon, so a K.O. on the last instant draws against the right edge.
        let left = (x(ko) - BAR_W / 2.0).min(rect.max.x - BAR_W).max(rect.min.x);
        let bar_rect = Rect::from_x_y_ranges(left..=left + BAR_W, top..=bottom);
        painter.rect_filled(bar_rect.intersect(rect), 0.0, bar);
    }
}

/// The small ribbon of a shortlist row: `width` wide (a width that is not a finite number above zero takes no
/// room) and 28 tall, with no K.O. window. It takes no input. Inside a disabled parent it draws its fills and bar
/// `FAINT`.
pub fn small(ui: &mut Ui, story: &StoryView, colors: [[u8; 3]; 2], width: f32) -> Response {
    let width = if width.is_finite() { width.max(0.0) } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(width, SMALL_H), Sense::hover());
    if ui.is_rect_visible(rect) {
        let mut painter = ui.painter().clone();
        let (fills, bar) = if ui.is_enabled() {
            ([rgb(colors[0]), rgb(colors[1])], TEXT_STRONG)
        } else {
            // Drawn FAINT here, at full strength, not faded by egui.
            painter.set_opacity(1.0);
            ([FAINT, FAINT], FAINT)
        };
        paint_in(&painter, rect, story, fills, bar, false);
    }
    response
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Rect, pos2, vec2};

    use super::*;
    use crate::model::story::AbilityMark;

    const FLUTE: Color32 = Color32::from_rgb(0xde, 0xe8, 0xf6);
    const TROMBONE: Color32 = Color32::from_rgb(0x78, 0xf0, 0xb8);

    /// The brief's story: FIGHT! at 3.6 s, Flute 21 HP ahead at 10 s, Trombone 21 ahead at 20 s, K.O. at 51 s.
    fn story() -> StoryView {
        StoryView {
            seed: 18433,
            winner: Some(1),
            t_ko: Some(51.0),
            winner_hp: 16.0,
            lead_changes: 1,
            score: 7.84,
            fits: true,
            misses: Vec::new(),
            t_fight: 3.6,
            lead: vec![(10.0, 21.0), (20.0, -21.0)],
            firsts: vec![AbilityMark {
                t: 12.0,
                who: 0,
                name: "Flutter tongue".to_string(),
            }],
        }
    }

    fn rect() -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 28.0))
    }

    /// The checks every mesh must pass: whole triangles, every index in range, every vertex finite and inside
    /// `rect`, the first colour above the centre line and the second below it.
    fn check(mesh: &Mesh, rect: Rect, colors: [Color32; 2]) {
        assert!(mesh.is_valid(), "every index points at a vertex");
        assert_eq!(mesh.indices.len() % 3, 0);
        let centre = rect.center().y;
        for v in &mesh.vertices {
            assert!(v.pos.x.is_finite() && v.pos.y.is_finite(), "{:?}", v.pos);
            assert!(rect.contains(v.pos), "{:?} outside {rect:?}", v.pos);
            if v.pos.y < centre {
                assert_eq!(v.color, colors[0], "above the line is the left fighter");
            }
            if v.pos.y > centre {
                assert_eq!(v.color, colors[1], "below the line is the right fighter");
            }
        }
    }

    #[test]
    fn time_and_x_convert_both_ways() {
        assert_eq!(x_of(30.5, 0.0, 1320.0), 660.0);
        assert_eq!(t_of(660.0, 0.0, 1320.0), 30.5);
        assert_eq!(t_of(-5.0, 0.0, 1320.0), 0.0);
        assert_eq!(t_of(99999.0, 0.0, 1320.0), 61.0);
        assert!((x_of(49.0, 0.0, 1320.0) - 1060.3).abs() < 0.1);
        assert!((x_of(53.0, 0.0, 1320.0) - x_of(49.0, 0.0, 1320.0) - 86.6).abs() < 0.1);
    }

    #[test]
    fn a_time_from_an_unusable_position_or_width_is_the_start() {
        assert_eq!(t_of(f32::NAN, 0.0, 1320.0), 0.0);
        assert_eq!(t_of(f32::INFINITY, 0.0, 1320.0), 61.0, "an infinity is the nearer end");
        assert_eq!(t_of(660.0, 0.0, 0.0), 0.0, "no width");
        assert_eq!(t_of(660.0, 0.0, -10.0), 0.0, "a negative width");
        assert_eq!(t_of(660.0, 0.0, f32::NAN), 0.0, "a width that is not a number");
        assert_eq!(t_of(660.0, f32::NAN, 1320.0), 0.0, "a left that is not a number");
    }

    #[test]
    fn a_42_hp_lead_fills_a_side() {
        assert_eq!(lead_height(42.0, 20.5), 19.5);
        assert_eq!(lead_height(100.0, 20.5), 19.5);
        assert_eq!(lead_height(21.0, 20.5), 9.75);
        assert_eq!(lead_height(0.0, 20.5), 0.0);
        assert_eq!(lead_height(-21.0, 20.5), 9.75, "the size of the lead, whoever has it");
    }

    #[test]
    fn a_height_is_never_negative_or_not_a_number() {
        assert_eq!(lead_height(f32::NAN, 20.5), 0.0, "no lead is no height");
        assert_eq!(
            lead_height(f32::INFINITY, 20.5),
            19.5,
            "an infinite lead fills the side"
        );
        assert_eq!(lead_height(21.0, 0.5), 0.0, "a ribbon 1 tall has no room");
        assert_eq!(lead_height(21.0, -3.0), 0.0, "nor one with a negative height");
        assert_eq!(lead_height(21.0, f32::NAN), 0.0, "nor one whose height is not a number");
        assert_eq!(lead_height(f32::INFINITY, f32::INFINITY), 0.0);
    }

    #[test]
    fn the_curve_runs_from_fight_through_each_hit_to_the_ko() {
        assert_eq!(
            curve(&story()),
            vec![(3.6, 0.0), (10.0, 21.0), (20.0, -21.0), (51.0, 0.0)]
        );
    }

    #[test]
    fn without_a_ko_the_curve_ends_at_the_last_hit() {
        let story = StoryView { t_ko: None, ..story() };
        assert_eq!(curve(&story), vec![(3.6, 0.0), (10.0, 21.0), (20.0, -21.0)]);
    }

    #[test]
    fn the_mesh_is_one_quad_per_side_per_step() {
        let colors = [FLUTE, TROMBONE];
        let mesh = mesh(&story(), rect(), colors);
        assert_eq!(mesh.indices.len() / 3, 2 * 3 * 2);
        check(&mesh, rect(), colors);
        let centre = rect().center().y;
        let top = mesh.vertices.iter().map(|v| v.pos.y).fold(f32::INFINITY, f32::min);
        let bottom = mesh.vertices.iter().map(|v| v.pos.y).fold(f32::NEG_INFINITY, f32::max);
        assert!((top - (centre - 6.5)).abs() < 1e-4, "21 HP is half of 13: {top}");
        assert!((bottom - (centre + 6.5)).abs() < 1e-4, "{bottom}");
    }

    /// The lead goes from +21 to -21 between 10 and 20 s: each side's fill meets the centre line at 15 s, where the
    /// straight line between the two points crosses it.
    #[test]
    fn a_lead_change_crosses_the_line_where_the_straight_line_does() {
        let rect = rect();
        let mesh = mesh(&story(), rect, [FLUTE, TROMBONE]);
        let x15 = x_of(15.0, rect.min.x, rect.width());
        let centre = rect.center().y;
        let at_crossing = mesh
            .vertices
            .iter()
            .filter(|v| (v.pos.x - x15).abs() < 1e-3)
            .collect::<Vec<_>>();
        assert!(!at_crossing.is_empty(), "the fills meet at 15 s");
        assert!(at_crossing.iter().all(|v| (v.pos.y - centre).abs() < 1e-4));
        // Between the two hits no fill reaches past the straight line.
        let x10 = x_of(10.0, rect.min.x, rect.width());
        let x20 = x_of(20.0, rect.min.x, rect.width());
        for v in mesh
            .vertices
            .iter()
            .filter(|v| v.pos.x > x10 + 1e-3 && v.pos.x < x20 - 1e-3)
        {
            assert!((v.pos.y - centre).abs() < 1e-4, "{:?}", v.pos);
        }
    }

    #[test]
    fn an_empty_lead_runs_along_the_line() {
        let story = StoryView {
            lead: Vec::new(),
            ..story()
        };
        assert_eq!(curve(&story), vec![(3.6, 0.0), (51.0, 0.0)]);
        let mesh = mesh(&story, rect(), [FLUTE, TROMBONE]);
        check(&mesh, rect(), [FLUTE, TROMBONE]);
        let centre = rect().center().y;
        assert!(mesh.vertices.iter().all(|v| v.pos.y == centre), "no fill");
    }

    #[test]
    fn a_single_point_draws_nothing() {
        let story = StoryView {
            lead: Vec::new(),
            t_ko: None,
            ..story()
        };
        assert_eq!(curve(&story), vec![(3.6, 0.0)]);
        assert!(mesh(&story, rect(), [FLUTE, TROMBONE]).is_empty());
    }

    #[test]
    fn hits_out_of_order_are_put_in_order() {
        let story = StoryView {
            lead: vec![(20.0, -21.0), (10.0, 21.0)],
            ..story()
        };
        assert_eq!(
            curve(&story),
            vec![(3.6, 0.0), (10.0, 21.0), (20.0, -21.0), (51.0, 0.0)]
        );
    }

    #[test]
    fn hits_that_cannot_be_used_are_left_out() {
        let story = StoryView {
            lead: vec![
                (f32::NAN, 5.0),
                (f32::INFINITY, 5.0),
                (12.0, f32::NAN),
                (13.0, f32::INFINITY),
                (14.0, f32::NEG_INFINITY),
                (1.0, 7.0),
                (52.0, 3.0),
                (10.0, 21.0),
            ],
            ..story()
        };
        assert_eq!(
            curve(&story),
            vec![(3.6, 0.0), (10.0, 21.0), (51.0, 0.0)],
            "not a number, infinite, before FIGHT! and after the K.O. are all left out"
        );
    }

    #[test]
    fn a_ko_that_cannot_be_used_counts_as_none() {
        for t_ko in [Some(f64::NAN), Some(f64::INFINITY), Some(70.0), Some(2.0), Some(-1.0)] {
            let story = StoryView { t_ko, ..story() };
            assert_eq!(curve(&story), vec![(3.6, 0.0), (10.0, 21.0), (20.0, -21.0)], "{t_ko:?}");
        }
        let at_end = StoryView {
            t_ko: Some(61.0),
            ..story()
        };
        assert_eq!(
            curve(&at_end).last(),
            Some(&(61.0, 0.0)),
            "a K.O. on the last instant is kept"
        );
    }

    #[test]
    fn hits_without_a_ko_stop_at_the_end_of_the_video() {
        let story = StoryView {
            t_ko: None,
            lead: vec![(10.0, 21.0), (61.0, 3.0), (64.0, 9.0)],
            ..story()
        };
        assert_eq!(curve(&story), vec![(3.6, 0.0), (10.0, 21.0), (61.0, 3.0)]);
    }

    #[test]
    fn a_fight_time_that_cannot_be_used_is_kept_inside_the_video() {
        let nan = StoryView {
            t_fight: f64::NAN,
            ..story()
        };
        assert_eq!(curve(&nan)[0], (0.0, 0.0), "not a number starts at 0");
        let early = StoryView {
            t_fight: -4.0,
            ..story()
        };
        assert_eq!(curve(&early)[0], (0.0, 0.0));
        let late = StoryView {
            t_fight: 99.0,
            t_ko: None,
            ..story()
        };
        assert_eq!(curve(&late), vec![(61.0, 0.0)], "every hit is before it");
    }

    #[test]
    fn the_ability_marks_do_not_change_the_ribbon() {
        let marks = StoryView {
            firsts: vec![
                AbilityMark {
                    t: -3.0,
                    who: 7,
                    name: String::new(),
                },
                AbilityMark {
                    t: f32::NAN,
                    who: 1,
                    name: "Pedal tone".to_string(),
                },
                AbilityMark {
                    t: 400.0,
                    who: 0,
                    name: "Snake dance".to_string(),
                },
            ],
            ..story()
        };
        let plain = mesh(&story(), rect(), [FLUTE, TROMBONE]);
        let marked = mesh(&marks, rect(), [FLUTE, TROMBONE]);
        assert_eq!(marked.vertices, plain.vertices);
        assert_eq!(marked.indices, plain.indices);
    }

    /// Leads far beyond 42 HP, hits on the same instant and a K.O. on the last instant: every vertex stays inside.
    #[test]
    fn huge_leads_and_crowded_hits_stay_inside_the_rect() {
        let story = StoryView {
            t_fight: 0.0,
            t_ko: Some(61.0),
            lead: vec![
                (5.0, 1e9),
                (5.0, -1e9),
                (5.0, 3.0),
                (30.0, f32::MAX),
                (30.0, -f32::MAX),
                (61.0, 50.0),
            ],
            ..story()
        };
        for rect in [rect(), Rect::from_min_size(pos2(-50.0, 3.0), vec2(1320.0, 41.0))] {
            let mesh = mesh(&story, rect, [FLUTE, TROMBONE]);
            assert!(!mesh.is_empty());
            check(&mesh, rect, [FLUTE, TROMBONE]);
            let top = mesh.vertices.iter().map(|v| v.pos.y).fold(f32::INFINITY, f32::min);
            assert!(top >= rect.min.y + 1.0, "a full side leaves 1 px: {top}");
        }
    }

    #[test]
    fn a_rect_with_no_room_gives_an_empty_mesh() {
        let rects = [
            Rect::from_min_size(pos2(10.0, 20.0), vec2(0.0, 28.0)),
            Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 0.0)),
            Rect::from_min_max(pos2(10.0, 20.0), pos2(5.0, 48.0)),
            Rect::from_min_max(pos2(10.0, 20.0), pos2(410.0, 10.0)),
            Rect::from_min_max(pos2(f32::NAN, 20.0), pos2(410.0, 48.0)),
            Rect::from_min_max(pos2(10.0, 20.0), pos2(f32::INFINITY, 48.0)),
            Rect::NOTHING,
            Rect::EVERYTHING,
        ];
        for rect in rects {
            assert!(mesh(&story(), rect, [FLUTE, TROMBONE]).is_empty(), "{rect:?}");
        }
    }

    #[test]
    fn a_ribbon_too_low_for_any_fill_lies_on_its_line() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 1.5));
        let mesh = mesh(&story(), rect, [FLUTE, TROMBONE]);
        check(&mesh, rect, [FLUTE, TROMBONE]);
        assert!(mesh.vertices.iter().all(|v| v.pos.y == rect.center().y));
    }
}
