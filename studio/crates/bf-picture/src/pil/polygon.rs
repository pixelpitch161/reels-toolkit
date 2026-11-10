//! `ImageDraw.polygon`: the edge-list scan fill of `Draw.c` (`ImagingDrawPolygon`, `polygon_generic`, `add_edge`)
//! and the 1 px outline.

use super::line::draw_line;
use super::{Plain, Sample, Surface, round_down_f32, round_up_f32, same_ink, truncated};
use crate::img::Img;

/// `Draw.c`'s `Edge`: one side of a polygon, from `(x0, y0)`, with its bounding box and its slope `dx` (columns
/// per row, a C `float`; 0 for a horizontal edge).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Edge {
    x0: i32,
    y0: i32,
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
    dx: f32,
}

impl Edge {
    /// `add_edge`.
    pub(crate) fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Edge {
        let (xmin, xmax) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (ymin, ymax) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        // `((float)(x1 - x0)) / (y1 - y0)`: both differences become floats, then one float division. (The
        // differences are taken in 64 bits so that absurd coordinates cannot overflow.)
        let dx = if y0 == y1 {
            0.0
        } else {
            (i64::from(x1) - i64::from(x0)) as f32 / (i64::from(y1) - i64::from(y0)) as f32
        };
        Edge {
            x0,
            y0,
            xmin,
            ymin,
            xmax,
            ymax,
            dx,
        }
    }

    /// Where the edge crosses row `y`: `(y - y0) * dx + x0`, a float product and a float sum.
    fn x_at(&self, y: i32) -> f32 {
        (i64::from(y) - i64::from(self.y0)) as f32 * self.dx + self.x0 as f32
    }
}

/// `draw_horizontal_lines`: in a blended fill, the horizontal edges of row `y` that start at or before the
/// position already drawn (`x_pos`; -1 when the row has no crossing at all).
fn draw_horizontal_lines<S: Surface>(s: &mut S, edges: &[Edge], x_pos: &mut i32, y: i32) {
    for e in edges {
        if e.ymin == y && e.ymin == e.ymax {
            let mut xmin = e.xmin;
            if *x_pos != -1 && *x_pos < xmin {
                // Line would be after the current position
                continue;
            }
            let xmax = e.xmax;
            if *x_pos > xmin {
                // Line would be partway through x_pos, so increase the starting point
                xmin = *x_pos;
                if xmax < xmin {
                    // Line would now end before it started
                    continue;
                }
            }
            s.hline(xmin, e.ymin, xmax);
            *x_pos = xmax.wrapping_add(1);
        }
    }
}

/// `polygon_generic`: fills the polygon whose sides are `edges`.
pub(crate) fn polygon_generic<S: Surface>(s: &mut S, edges: &[Edge]) {
    if edges.is_empty() {
        return;
    }
    let ysize = s.ysize();
    let mut ymin = ysize - 1;
    let mut ymax = 0;

    // Initialize the edge table and find polygon boundaries
    let mut edge_table: Vec<&Edge> = Vec::with_capacity(edges.len());
    for e in edges {
        if ymin > e.ymin {
            ymin = e.ymin;
        }
        if ymax < e.ymax {
            ymax = e.ymax;
        }
        if e.ymin == e.ymax {
            if !S::BLENDS {
                s.hline(e.xmin, e.ymin, e.xmax);
            }
            continue;
        }
        edge_table.push(e);
    }
    if ymin < 0 {
        ymin = 0;
    }
    if ymax > ysize {
        ymax = ysize;
    }

    // Process the edge table with a scan line searching for intersections
    let mut xx: Vec<f32> = Vec::with_capacity(edge_table.len() * 2);
    for y in ymin..=ymax {
        xx.clear();
        for (i, current) in edge_table.iter().enumerate() {
            if y < current.ymin || y > current.ymax {
                continue;
            }
            xx.push(current.x_at(y));
            let last = xx.len() - 1;
            if y == current.ymax && y < ymax {
                // Needed to draw consistent polygons
                xx.push(xx[last]);
            } else if (y == current.ymin || y == current.ymax) && current.dx != 0.0 {
                // Connect discontiguous corners
                for other in &edge_table[..i] {
                    if (y != other.ymin && y != other.ymax) || other.dx == 0.0 {
                        continue;
                    }
                    // Check if the two edges join to make a corner
                    if xx[last].round() == other.x_at(y).round() {
                        // Determine points from the edges on the next row
                        // Or if this is the last row, check the previous row
                        let offset = if y == current.ymax { -1 } else { 1 };
                        let adjacent = current.x_at(y + offset);
                        if y + offset >= other.ymin && y + offset <= other.ymax {
                            let adjacent_other = other.x_at(y + offset);
                            if xx[last] > adjacent + 1.0 && xx[last] > adjacent_other + 1.0 {
                                xx[last] = adjacent.max(adjacent_other).round() + 1.0;
                            } else if xx[last] < adjacent - 1.0 && xx[last] < adjacent_other - 1.0 {
                                xx[last] = adjacent.min(adjacent_other).round() - 1.0;
                            }
                            break;
                        }
                    }
                }
            }
        }
        // qsort with x_cmp; the crossings are never NaN.
        xx.sort_by(f32::total_cmp);
        if S::BLENDS {
            let mut x_pos: i32 = if xx.is_empty() { -1 } else { 0 };
            for &[left, right] in xx.as_chunks::<2>().0 {
                let x_end = round_down_f32(right);
                if x_end < x_pos {
                    // Line would be before the current position
                    continue;
                }
                draw_horizontal_lines(s, edges, &mut x_pos, y);
                if x_end < x_pos {
                    // Line would be before the current position
                    continue;
                }
                let mut x_start = round_up_f32(left);
                if x_pos > x_start {
                    // Line would be partway through x_pos, so increase the starting point
                    x_start = x_pos;
                    if x_end < x_start {
                        // Line would now end before it started
                        continue;
                    }
                }
                s.hline(x_start, y, x_end);
                x_pos = x_end.wrapping_add(1);
            }
            draw_horizontal_lines(s, edges, &mut x_pos, y);
        } else {
            for &[left, right] in xx.as_chunks::<2>().0 {
                s.hline(round_up_f32(left), y, round_down_f32(right));
            }
        }
    }
}

/// `ImagingDrawPolygon` with `fill` set: builds the edge list and fills it.
pub(crate) fn fill_polygon<S: Surface>(s: &mut S, xy: &[(i32, i32)]) {
    let count = xy.len();
    if count == 0 {
        return;
    }
    // Build edge list
    let mut e: Vec<Edge> = Vec::with_capacity(count);
    for i in 0..count - 1 {
        let (x0, y0) = xy[i];
        let (x1, y1) = xy[i + 1];
        if y0 == y1 && i != 0 && y0 == xy[i - 1].1 {
            // This is a horizontal line, that immediately follows another horizontal line
            let last = e.last_mut().expect("the first side is always added");
            if x1 > x0 && x0 > xy[i - 1].0 {
                // They are both increasing in x
                last.xmax = x1;
                continue;
            } else if x1 < x0 && x0 < xy[i - 1].0 {
                // They are both decreasing in x
                last.xmin = x1;
                continue;
            }
        }
        e.push(Edge::new(x0, y0, x1, y1));
    }
    let (first, last) = (xy[0], xy[count - 1]);
    if last != first {
        e.push(Edge::new(last.0, last.1, first.0, first.1));
    }
    polygon_generic(s, &e);
}

/// `ImagingDrawPolygon` without `fill`, width 1: the line of every side, the closing one included.
pub(crate) fn outline_polygon<S: Surface>(s: &mut S, xy: &[(i32, i32)]) {
    let count = xy.len();
    if count == 0 {
        return;
    }
    for i in 0..count - 1 {
        draw_line(s, xy[i].0, xy[i].1, xy[i + 1].0, xy[i + 1].1);
    }
    draw_line(s, xy[count - 1].0, xy[count - 1].1, xy[0].0, xy[0].1);
}

/// The points of a polygon truncated to pixels; `_draw_polygon` refuses fewer than two.
pub(crate) fn polygon_points(pts: &[(f64, f64)]) -> Vec<(i32, i32)> {
    assert!(
        pts.len() >= 2,
        "coordinate list must contain at least 2 coordinates (got {} point(s))",
        pts.len()
    );
    truncated(pts)
}

/// `ImageDraw.Draw(im).polygon(pts, fill=fill)`: a filled polygon. `fill` has one sample per channel.
/// Self-intersecting polygons fill by the even-odd rule of the scan; two points draw the line between them.
/// Panics on fewer than two points, as Pillow raises.
pub fn polygon<T: Sample>(img: &mut Img<T>, pts: &[(f64, f64)], fill: &[T]) {
    let xy = polygon_points(pts);
    fill_polygon(&mut Plain::new(img, fill), &xy);
}

/// `ImageDraw.Draw(im).polygon(pts, fill=fill, outline=outline)` and, with `fill` `None`,
/// `polygon(pts, outline=outline)`: the fill first, then a 1 px outline from point to point and back to the
/// first. As in Pillow the outline is skipped when its ink is the fill's ink.
pub fn polygon_outline<T: Sample>(img: &mut Img<T>, pts: &[(f64, f64)], fill: Option<&[T]>, outline: &[T]) {
    let xy = polygon_points(pts);
    if let Some(fill) = fill {
        fill_polygon(&mut Plain::new(img, fill), &xy);
        if same_ink(fill, outline) {
            return;
        }
    }
    outline_polygon(&mut Plain::new(img, outline), &xy);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(im: &Img<u8>) -> Vec<String> {
        (0..im.h)
            .map(|y| im.row(y).iter().map(|&v| if v != 0 { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn a_square_includes_its_far_edges() {
        let mut im: Img<u8> = Img::new(6, 5, 1);
        polygon(&mut im, &[(1.0, 1.0), (4.0, 1.0), (4.0, 3.0), (1.0, 3.0)], &[255]);
        assert_eq!(
            rows(&im),
            [".....", ".####", ".####", ".####", "....."].map(|r| format!("{r}."))
        );
    }

    #[test]
    fn coordinates_truncate_toward_zero() {
        let mut a: Img<u8> = Img::new(6, 5, 1);
        let mut b = a.clone();
        polygon(&mut a, &[(-0.9, -0.9), (2.99, -0.2), (2.5, 2.7), (-0.1, 2.2)], &[9]);
        polygon(&mut b, &[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)], &[9]);
        assert_eq!(a, b);
    }

    #[test]
    fn two_points_draw_a_line_and_one_point_a_dot() {
        let mut im: Img<u8> = Img::new(5, 3, 1);
        polygon(&mut im, &[(0.0, 1.0), (3.0, 1.0)], &[1]);
        assert_eq!(rows(&im), [".....", "####.", "....."]);
        let mut im: Img<u8> = Img::new(5, 3, 1);
        polygon(&mut im, &[(2.0, 1.0), (2.0, 1.0)], &[1]);
        assert_eq!(rows(&im), [".....", "..#..", "....."]);
    }

    #[test]
    fn the_outline_follows_the_fill_unless_it_is_the_same_ink() {
        let tri = [(1.0, 0.0), (5.0, 0.0), (3.0, 4.0)];
        let mut im: Img<u8> = Img::new(7, 5, 1);
        polygon_outline(&mut im, &tri, Some(&[1]), &[2]);
        assert_eq!(im.row(0), &[0, 2, 2, 2, 2, 2, 0]);
        assert_eq!(im.row(2), &[0, 0, 2, 1, 2, 0, 0]);
        let mut same: Img<u8> = Img::new(7, 5, 1);
        polygon_outline(&mut same, &tri, Some(&[1]), &[1]);
        let mut fill_only: Img<u8> = Img::new(7, 5, 1);
        polygon(&mut fill_only, &tri, &[1]);
        assert_eq!(same, fill_only);
    }

    /// The one place where `ROUND_UP`'s float sum shows. The edge from (0, 0) to (1, 82) crosses row 41 at
    /// `41 * (1 / 82)` in float arithmetic, which is 0.49999997, the largest float below one half. Adding `0.5F`
    /// in float rounds to 1.0, so the span starts at column 1; a double sum would start it at column 0. Pillow
    /// 12.3.0 draws exactly this on the reference PC (no case of `studio/reference/pillow` holds such an edge).
    #[test]
    fn an_edge_at_the_largest_float_below_half_starts_one_column_right() {
        let mut im: Img<u8> = Img::new(40, 84, 1);
        polygon(&mut im, &[(0.0, 0.0), (1.0, 82.0), (31.0, 82.0), (30.0, 0.0)], &[255]);
        assert_eq!(&im.row(40)[..3], &[255, 255, 255]);
        assert_eq!(&im.row(41)[..3], &[0, 255, 255]);
        assert_eq!(&im.row(42)[..3], &[0, 255, 255]);
        assert_eq!(&im.row(41)[29..33], &[255, 255, 0, 0]);
    }

    #[test]
    fn a_repeated_point_has_no_outline() {
        let mut im: Img<u8> = Img::new(4, 4, 1);
        polygon_outline(&mut im, &[(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)], None, &[5]);
        assert!(im.data.iter().all(|&v| v == 0));
    }

    #[test]
    #[should_panic(expected = "at least 2 coordinates")]
    fn one_point_is_refused() {
        let mut im: Img<u8> = Img::new(4, 4, 1);
        polygon(&mut im, &[(1.0, 1.0)], &[5]);
    }

    #[test]
    fn the_depth_buffer_takes_float_ink() {
        let mut im: Img<f32> = Img::filled(4, 3, &[1e6]);
        polygon(&mut im, &[(1.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0)], &[7.25]);
        assert_eq!(im.row(0), &[1e6, 7.25, 7.25, 1e6]);
        assert_eq!(im.row(2), &[1e6; 4]);
    }
}
