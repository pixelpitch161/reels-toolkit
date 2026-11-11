//! Fighter marks, chips and rows (interface reference, section 3 and 5.1): the fighter bar, the portrait, a picture
//! from the engine (a cover, a frame), the roster chip and the object row (shortlist rows, library tiles).
//!
//! The chips and rows paint their own look with a painter that egui's disabled fade does not touch, so a disabled
//! one is exactly `FAINT`; they paint their own focus ring (2 px `TEXT_STRONG`, 2 px outside) on a painter whose clip
//! rectangle reaches 4 px past the `Ui`'s, so the ring is not cut off at a panel's edge; and they name themselves for
//! the accessibility tree.
//!
//! Pictures from data (a sprite, a frame) are uploaded once as textures with nearest sampling and kept in one cache
//! in egui's temporary data (`ctx.data_mut`), keyed by a name. Freshness is by allocation (`Arc::ptr_eq`): a
//! texture is made again only when its pixels are another `Arc`, so a caller keeps one `Arc` per image (a new `Arc`
//! every frame means an upload every frame). The cache keeps at most `MAX_TEXTURES` (64) textures, dropping the
//! one drawn longest ago, except that it never drops one still on screen: with more than 64 on screen at once it
//! holds them all while they stay on screen and shrinks back afterwards (see `Cache`).

use std::sync::Arc;

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Color32, ColorImage, Context, FontId, Id, Key, Layout, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, TextWrapMode, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::buttons::{exact_painter, widget_ring};
use crate::model::roster::Sprite;
use crate::model::take::Frame;
use crate::theme::{
    FAINT, HOVER, MUTED, RADIUS_ROW, RAISED, ROW_H, SELECTED, TEXT, TEXT_STRONG, body, large_fill, rgb, small, strong,
};

/// A fighter bar's width.
const BAR_W: f32 = 4.0;
/// A fighter bar's corner.
const BAR_CORNER: f32 = 1.0;
/// Side padding of a chip or row.
const PAD_X: f32 = 10.0;
/// The chip's disc: its diameter, its ring and where it starts (after the bar and a gap of 8).
const CHIP_DISC: f32 = 14.0;
const CHIP_DISC_RING: f32 = 2.0;
const CHIP_DISC_LEFT: f32 = 12.0;
/// The gap after the disc and before the side label.
const CHIP_GAP: f32 = 8.0;
/// The disc's fill: the fighter's colour at one third opacity.
const CHIP_DISC_ALPHA: u8 = 85;
/// The ring of a selected chip or row, drawn inside it.
const SELECTED_RING: f32 = 1.5;
/// The ring around a portrait's disc.
const PORTRAIT_RING: f32 = 3.0;
/// How many picture textures the cache keeps at most.
const MAX_TEXTURES: usize = 64;
/// What hovering a chip that is not built says.
const NOT_BUILT: &str = "Not built yet. It arrives in an update.";

/// The keyboard focus ring around `rect` (corner 8 + 2), 2 px `TEXT_STRONG`, 2 px outside, on a painter that may
/// paint 4 px past `ui`'s clip rectangle.
fn focus_ring(ui: &Ui, rect: Rect) {
    widget_ring(ui, rect, f32::from(RADIUS_ROW));
}

/// The selected look of a chip, an object row or a chord loop, with corners `corner`: a `SELECTED` fill and a 1.5 px
/// ring drawn inside, `TEXT` (`FAINT` when `enabled` is false). Returns the colour its text takes: `TEXT_STRONG`, or
/// `FAINT` when disabled.
pub(crate) fn paint_selected(painter: &Painter, rect: Rect, corner: u8, enabled: bool) -> Color32 {
    painter.rect_filled(rect, corner, SELECTED);
    let ring = if enabled { TEXT } else { FAINT };
    painter.rect_stroke(rect, corner, Stroke::new(SELECTED_RING, ring), StrokeKind::Inside);
    if enabled { TEXT_STRONG } else { FAINT }
}

/// `rect` with its corners on whole physical pixels.
fn snap(rect: Rect, ppp: f32) -> Rect {
    let px = |p: Pos2| pos2((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
    Rect::from_min_max(px(rect.min), px(rect.max))
}

/// A fighter bar: 4 wide and `height` tall in the fighter's full colour, corner 1. Decoration only: it has no name,
/// and inside a disabled `Ui` it takes egui's disabled fade like any other content.
pub fn fighter_bar(ui: &mut Ui, color: [u8; 3], height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(BAR_W, height), Sense::hover());
    if ui.is_rect_visible(rect) {
        let rect = snap(rect, ui.ctx().pixels_per_point());
        ui.painter().rect_filled(rect, BAR_CORNER, rgb(color));
    }
    response
}

/// A fighter's portrait, `size` square, named `name` (an image). With a usable sprite it is the sprite, drawn with
/// nearest sampling at the largest whole-number scale that fits (scaled down to fit when the sprite is larger than
/// `size`), centred on whole pixels. Without one, or when the sprite is empty, its buffer does not hold `w * h * 4`
/// bytes or it is larger than a texture can be, it is a disc filled with `large_fill(color)` inside a 3 px ring of
/// the full colour. A `size` that is not a positive number takes no space and draws nothing.
///
/// The sprite's texture is cached under `name` and is fresh while `sprite.rgba` is the same allocation
/// (`Arc::ptr_eq`): keep one `Arc` per sprite, since a new `Arc` each frame uploads it each frame. The cache holds
/// 64 textures, or all those on screen when more are (see the module's notes).
///
/// It is decoration: inside a disabled `Ui` it takes egui's disabled fade like any other content.
pub fn portrait(ui: &mut Ui, color: [u8; 3], sprite: Option<&Sprite>, size: f32, name: &str) -> Response {
    let drawable = size.is_finite() && size > 0.0;
    let side = if drawable { size } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, name));
    if !drawable || !ui.is_rect_visible(rect) {
        return response;
    }
    let ctx = ui.ctx().clone();
    let usable = sprite.filter(|s| usable(&ctx, s.w, s.h, 4, s.rgba.len()));
    match usable {
        Some(sprite) => {
            let texture = texture(
                &ctx,
                format!("portrait {name}"),
                &sprite.rgba,
                [sprite.w, sprite.h],
                || ColorImage::from_rgba_unmultiplied([sprite.w, sprite.h], &sprite.rgba),
            );
            let (w, h) = (sprite.w as f32, sprite.h as f32);
            let whole = (size / w.max(h)).floor();
            let scale = if whole >= 1.0 { whole } else { size / w.max(h) };
            let drawn = vec2(w * scale, h * scale);
            let at = snap(
                Rect::from_min_size(rect.center() - drawn / 2.0, drawn),
                ctx.pixels_per_point(),
            );
            let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
            ui.painter().image(texture.id(), at, uv, Color32::WHITE);
        }
        None => {
            let painter = ui.painter();
            let radius = size / 2.0;
            painter.circle_filled(rect.center(), radius, large_fill(color));
            // epaint strokes a circle outside its radius, so the ring's radius is its inner edge.
            painter.circle_stroke(
                rect.center(),
                radius - PORTRAIT_RING,
                Stroke::new(PORTRAIT_RING, rgb(color)),
            );
        }
    }
    response
}

/// A picture from the engine (a cover, a video frame) drawn into `size` points with nearest sampling, its corners on
/// whole pixels, named `name` (an image). A frame that is empty, whose buffer does not hold `w * h * 3` bytes or
/// that is larger than a texture can be is drawn as a `RAISED` block of `size`. A `size` with a side that is not a
/// positive number takes no space and draws nothing.
///
/// Its texture is cached under `key` and is fresh while `frame.rgb` is the same allocation (`Arc::ptr_eq`): keep
/// one `Arc` per picture, since a new `Arc` each frame uploads it each frame. The cache holds 64 textures, or all
/// those on screen when more are (see the module's notes).
///
/// It is decoration: inside a disabled `Ui` it takes egui's disabled fade like any other content.
pub fn picture(ui: &mut Ui, key: &str, frame: &Frame, size: Vec2, name: &str) -> Response {
    let drawable = size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0;
    let size = if drawable { size } else { Vec2::ZERO };
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, name));
    if !drawable || !ui.is_rect_visible(rect) {
        return response;
    }
    let ctx = ui.ctx().clone();
    let at = snap(rect, ctx.pixels_per_point());
    let (w, h) = (frame.w as usize, frame.h as usize);
    if usable(&ctx, w, h, 3, frame.rgb.len()) {
        let texture = texture(&ctx, format!("picture {key}"), &frame.rgb, [w, h], || {
            ColorImage::from_rgb([w, h], &frame.rgb)
        });
        let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        ui.painter().image(texture.id(), at, uv, Color32::WHITE);
    } else {
        ui.painter().rect_filled(at, 0.0, RAISED);
    }
    response
}

/// Whether a `w` x `h` picture of `channels` bytes per pixel in a buffer of `len` bytes can be made a texture: not
/// empty, the buffer exactly the size, and no side over the largest texture side.
fn usable(ctx: &Context, w: usize, h: usize, channels: usize, len: usize) -> bool {
    let max_side = ctx.input(|i| i.max_texture_side);
    let exact = w.checked_mul(h).and_then(|n| n.checked_mul(channels)) == Some(len);
    w > 0 && h > 0 && w <= max_side && h <= max_side && exact
}

/// One cached texture: what it was made from, the pass it was last drawn in and, for recency, the cache's call
/// count when it was last drawn.
#[derive(Clone)]
struct Entry<T> {
    key: String,
    pixels: Arc<[u8]>,
    size: [usize; 2],
    handle: T,
    pass: u64,
    used: u64,
}

/// The picture textures. Generic over the handle so its rules can be tested without a context. Each entry keeps its
/// pixels' `Arc`, so their address cannot be reused by other pixels while cached.
///
/// The bound: while there are more than `MAX_TEXTURES`, entries drawn neither in the current pass nor in the pass
/// before it are dropped, the one drawn longest ago first (recency is the call count, so entries drawn in one pass
/// are ordered too). An entry on screen is never dropped, so when more than `MAX_TEXTURES` pictures are on screen at
/// once the cache grows to hold them all for as long as they stay there (nothing is uploaded again), and it shrinks
/// back to `MAX_TEXTURES` on the second pass after some of them leave the screen. The overflow is never re-made.
#[derive(Clone)]
struct Cache<T> {
    entries: Vec<Entry<T>>,
    /// Counts lookups and stores, so that entries of one pass are still ordered by recency.
    clock: u64,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache {
            entries: Vec::new(),
            clock: 0,
        }
    }
}

/// What the cache holds for a key.
#[derive(Debug, PartialEq)]
enum Found<T> {
    /// A texture made from these very pixels.
    Fresh(T),
    /// A texture for this key made from other pixels: set it to the new ones.
    Stale(T),
    Missing,
}

impl<T: Clone> Cache<T> {
    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// The texture for `key` and `pixels`, marking it drawn in `pass` when it is fresh. Trims the cache first.
    fn find(&mut self, key: &str, pixels: &Arc<[u8]>, size: [usize; 2], pass: u64) -> Found<T> {
        self.trim(pass);
        let used = self.tick();
        match self.entries.iter_mut().find(|e| e.key == key) {
            Some(e) if Arc::ptr_eq(&e.pixels, pixels) && e.size == size => {
                e.pass = pass;
                e.used = used;
                Found::Fresh(e.handle.clone())
            }
            Some(e) => Found::Stale(e.handle.clone()),
            None => Found::Missing,
        }
    }

    /// Keeps `handle` for `key`, made from `pixels`, drawn in `pass`; then trims the cache.
    fn put(&mut self, key: String, pixels: Arc<[u8]>, size: [usize; 2], handle: T, pass: u64) {
        let entry = Entry {
            key,
            pixels,
            size,
            handle,
            pass,
            used: self.tick(),
        };
        match self.entries.iter_mut().find(|e| e.key == entry.key) {
            Some(old) => *old = entry,
            None => self.entries.push(entry),
        }
        self.trim(pass);
    }

    /// While there are more than `MAX_TEXTURES`, drops the entry drawn longest ago among those drawn neither in
    /// `pass` nor in the pass before it (so what is on screen is never dropped between one pass and the next).
    fn trim(&mut self, pass: u64) {
        while self.entries.len() > MAX_TEXTURES {
            let oldest = (0..self.entries.len())
                .filter(|&i| self.entries[i].pass.saturating_add(1) < pass)
                .min_by_key(|&i| self.entries[i].used);
            match oldest {
                Some(i) => {
                    self.entries.remove(i);
                }
                None => break,
            }
        }
    }
}

/// The texture for `key` made from `pixels` (`size` pixels, `image` building it), from the cache when it holds one
/// made from these very pixels, else uploaded (or set on the key's old texture) and cached.
fn texture(
    ctx: &Context,
    key: String,
    pixels: &Arc<[u8]>,
    size: [usize; 2],
    image: impl FnOnce() -> ColorImage,
) -> TextureHandle {
    let id = Id::unique("bf-app picture textures");
    let pass = ctx.cumulative_pass_nr();
    let found = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache<TextureHandle>>(id)
            .find(&key, pixels, size, pass)
    });
    let handle = match found {
        Found::Fresh(handle) => return handle,
        Found::Stale(mut handle) => {
            handle.set(image(), TextureOptions::NEAREST);
            handle
        }
        Found::Missing => ctx.load_texture(format!("bf-app {key}"), image(), TextureOptions::NEAREST),
    };
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache<TextureHandle>>(id)
            .put(key, pixels.clone(), size, handle.clone(), pass);
    });
    handle
}

/// What a chip got this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChipOut {
    /// Clicked (or Space or Enter while focused): the left corner.
    pub left: bool,
    /// Right-clicked (or the menu key or Shift+F10 while focused): the right corner.
    pub right: bool,
}

/// The outline of `rect` with corners `r`, clockwise on screen, each corner in `steps` segments.
fn rounded_outline(rect: Rect, r: f32, steps: usize) -> Vec<Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let corners = [
        (pos2(rect.min.x + r, rect.min.y + r), PI),
        (pos2(rect.max.x - r, rect.min.y + r), 1.5 * PI),
        (pos2(rect.max.x - r, rect.max.y - r), 0.0),
        (pos2(rect.min.x + r, rect.max.y - r), FRAC_PI_2),
    ];
    let mut points = Vec::with_capacity(4 * (steps + 1));
    for (centre, start) in corners {
        for i in 0..=steps {
            let a = start + FRAC_PI_2 * i as f32 / steps as f32;
            points.push(centre + r * vec2(a.cos(), a.sin()));
        }
    }
    points
}

/// The part of the convex polygon `points` left of `x` (Sutherland and Hodgman, one edge).
fn left_of(points: &[Pos2], x: f32) -> Vec<Pos2> {
    let mut out = Vec::with_capacity(points.len() + 2);
    for (i, &p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        let (p_in, q_in) = (p.x <= x, q.x <= x);
        if p_in {
            out.push(p);
        }
        if p_in != q_in {
            let t = (x - p.x) / (q.x - p.x);
            out.push(pos2(x, p.y + t * (q.y - p.y)));
        }
    }
    out
}

/// A roster chip, `width` wide and 30 tall, corner 8: a 4 x 30 bar in the fighter's colour along its left edge (cut
/// by the corners), a 14 round disc (2 px ring in the colour, filled with it at one third opacity), the name, and
/// at the right "Left" or "Right" (`small`, `MUTED`) when `side` is `Some(0)` or `Some(1)` (any other side counts as
/// not chosen). A chosen chip has the selected look: `SELECTED` fill, a 1.5 px `TEXT` ring inside, the name in Plex
/// Sans Medium `TEXT_STRONG`. A name too long for the chip ends in an ellipsis.
///
/// Its accessible name is "Flute", "Flute, left corner" or "Flute, right corner". A chip that is not `available`, or
/// sits in a disabled `Ui`, is drawn `FAINT` (bar, disc and name) and takes no clicks; one not available says "Not
/// built yet. It arrives in an update." on hover.
pub fn roster_chip(
    ui: &mut Ui,
    display: &str,
    color: [u8; 3],
    side: Option<usize>,
    available: bool,
    width: f32,
) -> ChipOut {
    let enabled = available && ui.is_enabled();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, ROW_H), sense);
    let corner = match side {
        Some(0) => Some(("Left", "left")),
        Some(1) => Some(("Right", "right")),
        _ => None,
    };
    let name = match corner {
        Some((_, which)) => format!("{display}, {which} corner"),
        None => display.to_owned(),
    };
    response.widget_info(|| WidgetInfo::selected(Role::Button, enabled, corner.is_some(), &name));
    if !available {
        response = response.on_hover_text(NOT_BUILT).on_disabled_hover_text(NOT_BUILT);
    }

    if ui.is_rect_visible(rect) {
        let painter = exact_painter(ui);
        let chosen = corner.is_some();
        if chosen {
            paint_selected(&painter, rect, RADIUS_ROW, enabled);
        } else if enabled && response.hovered() {
            painter.rect_filled(rect, RADIUS_ROW, HOVER);
        }

        let mark = if enabled { rgb(color) } else { FAINT };
        let outline = rounded_outline(rect, f32::from(RADIUS_ROW), 8);
        painter.add(Shape::convex_polygon(
            left_of(&outline, rect.min.x + BAR_W),
            mark,
            Stroke::NONE,
        ));
        let centre = pos2(rect.min.x + CHIP_DISC_LEFT + CHIP_DISC / 2.0, rect.center().y);
        let [r, g, b, _] = mark.to_array();
        painter.circle_filled(
            centre,
            CHIP_DISC / 2.0,
            Color32::from_rgba_unmultiplied(r, g, b, CHIP_DISC_ALPHA),
        );
        painter.circle_stroke(
            centre,
            CHIP_DISC / 2.0 - CHIP_DISC_RING,
            Stroke::new(CHIP_DISC_RING, mark),
        );

        let mut right = rect.max.x - PAD_X;
        if let Some((label, _)) = corner {
            let galley = painter.layout_no_wrap(label.to_owned(), small(), MUTED);
            let at = pos2(right - galley.size().x, rect.center().y - galley.size().y / 2.0).round();
            right = at.x - CHIP_GAP;
            let label_rect = Rect::from_min_size(at, galley.size());
            let node = ui.interact(label_rect, response.id.with("side"), Sense::hover());
            node.widget_info(|| WidgetInfo::labeled(Role::Label, true, label));
            painter.galley(at, galley, MUTED);
        }
        let left = rect.min.x + CHIP_DISC_LEFT + CHIP_DISC + CHIP_GAP;
        let (font, colour) = match (enabled, chosen) {
            (false, _) => (body(), FAINT),
            (true, true) => (strong(), TEXT_STRONG),
            (true, false) => (body(), TEXT),
        };
        paint_name(&painter, display, font, colour, left, right, rect);
    }

    if response.has_focus() {
        focus_ring(ui, rect);
    }
    let keyed_right = response.has_focus()
        && ui.input(|i| i.key_pressed(Key::Menu) || (i.modifiers.shift_only() && i.key_pressed(Key::F10)));
    ChipOut {
        left: enabled && response.clicked(),
        right: enabled && (response.secondary_clicked() || keyed_right),
    }
}

/// `text` on one line from `left` to at most `right`, in the middle of `rect`'s height, ending in an ellipsis when
/// it does not fit, and never painted past `right`.
fn paint_name(painter: &Painter, text: &str, font: FontId, colour: Color32, left: f32, right: f32, rect: Rect) {
    let max_width = (right - left).max(0.0);
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, colour);
    job.wrap = TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    let at = pos2(left, rect.center().y - galley.size().y / 2.0).round();
    let area = Rect::from_x_y_ranges(left..=right.max(left), rect.y_range());
    painter.with_clip_rect(area).galley(at, galley, colour);
}

/// An object row (a shortlist row, a library tile): as wide as the space left and `height` tall, corner 8, padded 10
/// at each side, with `add` laying out its content left to right, centred in its height, clipped to the row. The
/// whole row is one control named `name` (a button that is selected or not): clicking anywhere on it, or Space or
/// Enter while it has the focus, clicks it. Selected: `SELECTED` fill, a 1.5 px `TEXT` ring inside, and its text
/// `TEXT_STRONG`; otherwise transparent, `HOVER` when hovered. In a disabled `Ui` it takes no clicks, its ring is
/// `FAINT` and its text `FAINT`. Labels inside it cannot be selected, so they never take its clicks, and they do
/// not wrap: a label too long for the row is cut with an ellipsis.
pub fn object_row<R>(
    ui: &mut Ui,
    name: &str,
    selected: bool,
    height: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> (Response, R) {
    let enabled = ui.is_enabled();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), sense);
    response.widget_info(|| WidgetInfo::selected(Role::Button, enabled, selected, name));
    if ui.is_rect_visible(rect) {
        let painter = exact_painter(ui);
        if selected {
            paint_selected(&painter, rect, RADIUS_ROW, enabled);
        } else if enabled && response.hovered() {
            painter.rect_filled(rect, RADIUS_ROW, HOVER);
        }
    }
    let inside = rect.shrink2(vec2(PAD_X, 0.0));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    let style = child.style_mut();
    style.interaction.selectable_labels = false;
    style.wrap_mode = Some(TextWrapMode::Truncate);
    style.visuals.override_text_color = Some(match (enabled, selected) {
        (false, _) => FAINT,
        (true, true) => TEXT_STRONG,
        (true, false) => TEXT,
    });
    let inner = add(&mut child);
    if response.has_focus() {
        focus_ring(ui, rect);
    }
    (response, inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(n: u8) -> Arc<[u8]> {
        Arc::from(vec![n; 3])
    }

    #[test]
    fn the_same_pixels_are_found_fresh_and_other_pixels_stale() {
        let mut cache = Cache::<u32>::default();
        let first = pixels(1);
        assert_eq!(cache.find("a", &first, [1, 1], 0), Found::Missing);
        cache.put("a".into(), first.clone(), [1, 1], 7, 0);
        assert_eq!(cache.find("a", &first, [1, 1], 1), Found::Fresh(7));
        assert_eq!(
            cache.find("a", &first.clone(), [1, 1], 2),
            Found::Fresh(7),
            "a clone is the same allocation"
        );
        let same_values = pixels(1);
        assert_eq!(
            cache.find("a", &same_values, [1, 1], 3),
            Found::Stale(7),
            "another allocation is made again"
        );
        assert_eq!(
            cache.find("a", &first, [3, 1], 3),
            Found::Stale(7),
            "another size is made again"
        );
        cache.put("a".into(), same_values.clone(), [1, 1], 7, 3);
        assert_eq!(cache.entries.len(), 1, "a key has one entry");
        assert_eq!(cache.find("a", &same_values, [1, 1], 4), Found::Fresh(7));
        assert_eq!(
            cache.find("b", &same_values, [1, 1], 4),
            Found::Missing,
            "keys are separate"
        );
    }

    #[test]
    fn the_cache_drops_the_one_used_longest_ago() {
        let mut cache = Cache::<usize>::default();
        let all: Vec<_> = (0..=MAX_TEXTURES).map(|i| pixels(i as u8)).collect();
        for (i, p) in all.iter().enumerate().take(MAX_TEXTURES) {
            cache.put(format!("k{i}"), p.clone(), [1, 1], i, i as u64);
        }
        assert!(
            matches!(cache.find("k0", &all[0], [1, 1], 100), Found::Fresh(0)),
            "k0 is used again"
        );
        cache.put(
            format!("k{MAX_TEXTURES}"),
            all[MAX_TEXTURES].clone(),
            [1, 1],
            MAX_TEXTURES,
            101,
        );
        assert_eq!(cache.entries.len(), MAX_TEXTURES, "bounded");
        assert_eq!(
            cache.find("k1", &all[1], [1, 1], 102),
            Found::Missing,
            "k1 was used longest ago"
        );
        assert!(matches!(cache.find("k0", &all[0], [1, 1], 102), Found::Fresh(0)));
        assert!(matches!(
            cache.find(&format!("k{MAX_TEXTURES}"), &all[MAX_TEXTURES], [1, 1], 102),
            Found::Fresh(_)
        ));
    }

    /// Draws `count` pictures in one pass, in order, the way `texture` does; returns how many were uploaded.
    fn draw_pass(cache: &mut Cache<usize>, all: &[Arc<[u8]>], count: usize, pass: u64) -> usize {
        let mut uploads = 0;
        for (i, p) in all.iter().enumerate().take(count) {
            if cache.find(&format!("k{i}"), p, [1, 1], pass) != Found::Fresh(i) {
                uploads += 1;
                cache.put(format!("k{i}"), p.clone(), [1, 1], i, pass);
            }
        }
        uploads
    }

    #[test]
    fn pictures_on_screen_are_never_made_again_however_many() {
        for count in [MAX_TEXTURES - 1, MAX_TEXTURES, MAX_TEXTURES + 1, 80] {
            let mut cache = Cache::<usize>::default();
            let all: Vec<_> = (0..count).map(|i| pixels(i as u8)).collect();
            assert_eq!(
                draw_pass(&mut cache, &all, count, 1),
                count,
                "{count}: the first pass uploads each"
            );
            for pass in 2..6 {
                assert_eq!(
                    draw_pass(&mut cache, &all, count, pass),
                    0,
                    "{count}: pass {pass} uploads none"
                );
            }
            assert_eq!(cache.entries.len(), count, "{count}: all on screen are held");
            // Ten stay on screen: the cache shrinks back to its bound on the next pass and keeps those ten.
            assert_eq!(
                draw_pass(&mut cache, &all, 10, 6),
                0,
                "{count}: the ten that stay are not made again"
            );
            assert_eq!(
                cache.entries.len(),
                count,
                "{count}: last pass's pictures are kept one pass"
            );
            assert_eq!(draw_pass(&mut cache, &all, 10, 7), 0);
            assert_eq!(
                cache.entries.len(),
                count.min(MAX_TEXTURES),
                "{count}: back within the bound"
            );
            assert_eq!(draw_pass(&mut cache, &all, 10, 8), 0);
        }
    }

    #[test]
    fn within_one_pass_the_first_drawn_is_the_oldest() {
        let mut cache = Cache::<usize>::default();
        let all: Vec<_> = (0..=MAX_TEXTURES).map(|i| pixels(i as u8)).collect();
        draw_pass(&mut cache, &all, MAX_TEXTURES, 1);
        let last = MAX_TEXTURES;
        cache.put(format!("k{last}"), all[last].clone(), [1, 1], last, 3);
        assert_eq!(cache.entries.len(), MAX_TEXTURES);
        assert_eq!(
            cache.find("k0", &all[0], [1, 1], 3),
            Found::Missing,
            "k0 was drawn first"
        );
        assert_eq!(cache.find("k1", &all[1], [1, 1], 3), Found::Fresh(1));
    }

    #[test]
    fn the_bar_is_the_rounded_outline_left_of_4() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(120.0, 30.0));
        let bar = left_of(&rounded_outline(rect, 8.0, 8), 4.0);
        assert!(bar.iter().all(|p| p.x <= 4.0 + 1e-4 && p.x >= -1e-4));
        assert!(
            bar.iter().any(|p| (p.y - 15.0).abs() < 8.0 && p.x.abs() < 1e-4),
            "the straight left edge"
        );
        let top = bar.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        assert!(top > 1.0 && top < 3.0, "the corner cuts the top: {top}");
        assert!(left_of(&[], 4.0).is_empty());
    }
}
