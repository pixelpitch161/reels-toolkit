# Engine 2 style port (E7)

Aedan's rule (verbatim): "I want to make sure the art style stays the same, just higher fidelity and improved with
normal maps for the lighting. The point of the new engine was more advanced effects, not revamping the entire
style."

This page maps each element of the current engine's picture (`bf-picture`) to its Engine 2 equivalent. The values
are not copied: Engine 2 reads them from the current engine's own constants and functions (`bf_picture::scene`,
`bf_picture::lighting::shade`, `bf_picture::ring::state::camera_keys`), so the two cannot drift apart. The current
engine's output is unchanged; the only change to `bf-picture` is that `camera_keys` is now public.

Comparison stills (current | Engine 2 before E7 | Engine 2 after E7) are in
`/mnt/project-files/ballfight-studio/engine2/style/` (`e7_calm_current_vs_engine2.png`,
`e7_hit_current_before_after.png`, `e7_ko_current_before_after.png`). The `engine2_style` example renders the
TEST DUMMY's stills:

```
cargo build -p bf-export --example engine2_style
./target/debug/examples/engine2_style <folder>            # lists the heavy hits and the K.O.
./target/debug/examples/engine2_style <folder> 300 701 1490
./target/debug/ballfight frame --frame 700 --up --out <file> fights/euphonium_vs_trombone.toml
```

## Space and camera

| Current engine | Engine 2 |
|---|---|
| World = ring frame turned 45 degrees about Y (`scene::to_world`); the ring is a diamond, fighter 0's corner (`RED`, ring `(-P, +P)`) on the left, fighter 1's (`BLUE`, `(P, -P)`) on the right | Render space is the ring frame: render `(x, z)` = ring `(sx, sz)`, sim `(x, y)` -> render `(x, -y)`. `stage::from_world` / `bf2_direct::base::from_world` take a current world point to render space (`scene::to_ring`) |
| `camera_keys(t_ko)`: distance 27.5, pitch 58, fov 36, aim `[0.12, 0.25, 0]`, optical centre `SY = 288` (32 rows above the middle), a slow drift and a push-in to 26.7 at the K.O. | `bf2_direct::base::base_rig(t, t_ko)`: the same keys interpolated by `bf_picture::camera::interp_keys`; the off-centre optical centre becomes a pitch down onto the middle row's ray (within 3 px of the current picture at 1080 x 1920, tested on the ring's corners and post tops) |
| no director | Director shots (punch-in, follow, low angle, orbit, whip, shake) are offsets around the base pose, scaled by `DirectCfg::intensity`, default 0.2: very subtle (Aedan) |

## The ring

| Element | Current engine | Engine 2 |
|---|---|---|
| Canvas | `canvas_albedo(sx, sz)`: `CANVAS [104, 98, 142]`, the rings at r 1.55 / 1.32, the five-lobe star, the rim `CANVAS_RIM [70, 58, 104]`, the white band at `AP - 0.08` | The same function baked into a 1024 x 1024 albedo map over the apron (`stage::canvas_map`, tested texel by texel), on a plane of half-size `AP = 3.62`, matte, plus a woven normal map (`texture::weave_height`, 64 threads, repeated twice across) |
| Canvas light | `amb x 0.75 + cone x lam x 0.86` (cone soft 19 to 27 degrees) | Ambient `AMBIENT` (the current `Lights` ambient `[0.36, 0.33, 0.44]` in linear light) and the spot (`SPOT`, `SPOT_AIM`, 27 degrees, inner 21.6, shadowed), colour `[1.3, 1.22, 1.3]` tuned against the current frames |
| Skirt | `Scene::skirt_faces`: `SKIRT [34, 30, 58]` with the band `SKIRT_BAND [150, 140, 184]` | The same faces, each coloured with the current `shade` result, baked for the ambient |
| Posts | Steel pole r 0.085 `[190, 190, 204]`, height `POST_H 1.42`, cap r 0.11 `[220, 220, 230]`, a cushion r 0.16 at each rope height in the corner's colour (fighter colour x 0.9 on `RED` / `BLUE`, `[236, 232, 222]` on the others) | The same sizes and colours as toon meshes, lifted (`SET_LIFT`) so the ambient shows them near the current brightness where the spot's cone does not reach (the current engine lights them with a directional key); cushions get a faint hammered normal map |
| Ropes | Three at `ROPE_Y [0.42, 0.80, 1.18]`, sag `0.035 sin(pi u)`, cream `ROPE_COL [236, 230, 222]`, the middle one in its side's fighter colour x 0.92; the give where a ball hits (5 Hz, 0.22 s decay, 0.9 s) | The same heights, sag, colours (`SIDE_FIGHTER`: the sides touching `RED` are fighter 0's); radius 0.028; the arena track's wobble now uses the current give (5 Hz, e-fold 0.22 s, let go at 0.9 s, at most 0.16) |
| Corner lights | Point lights over the `RED` and `BLUE` post tops (`POST_H + 0.35`), `light_rgb x 1.1`, pulsing on the beat; they light the floor (`floor`: `h / d / (1 + (d / 4.7)^2)`) | The same lights (range 4.2) and a glow; the floor spill is an `Additive` mesh weighted by the same formula, in linear light about the floor's own light |

## The arena

| Element | Current engine | Engine 2 |
|---|---|---|
| Floor | `floor`: `ARENA_FLOOR [46, 40, 62]` x `[0.20, 0.18, 0.26] + 0.55 e^(-(r - 4.6) / 3)` | A disc at `FLOOR_Y -1.15`, unlit, its vertices coloured by the same formula (the faint checks left out) |
| LED boards | Around `BARRIER_R 5.55`, `BARRIER_H 0.72`: each half in its fighter's colour (`sin(am) < 0`: fighter 0), a chase (`fmod(am / 2 pi x 16 + beat, 4) < 2` full, else `x 0.42 + 40`), pulse `0.55 + 0.45 beat`, fill `x 0.55 + 18`, top edge `x 0.9 + 60` | 64 emissive boards with the same colours, chase, pulse, fill and edge; a flash in the flashing fighter's colour on top (Engine 2's arena reaction) |
| Stands | `N_ROWS 10` tiers from `STAND_R0 6.2`, `ROW_W 0.82`, `ROW_H 0.40`; tops `[54, 46, 74]`, risers `[40, 34, 58]`, shaded then x 0.55 | The same tiers as one mesh (160 segments round), each face's colour the current `shade` result x 0.55, baked for the ambient (matte) |
| Crowd | `Scene::new(col).crowd` (seeded): head r 0.15 at `0.55 ht`, body ellipse 3.8 x 2.8 head radii; body `CROWD_COLS[ci] x tone x 0.48`, tone `0.78 + 0.42 light_rgb` on each fighter's side; head x 0.62 of the body; phones for `ph < 0.07`; a one-pixel hop | The same people at the same seats (render positions via `from_world`), each a body ellipsoid and head turned to the base camera so it reads as the same figure, the same colours baked for the ambient (matte, so it keeps a little roundness); emissive phones twinkling; the arena's crowd jump at most 0.08 |
| Spot | At `SPOT [0, 9.5, -1.4]`, aimed at `SPOT_AIM`, cone 19 to 27 degrees | A shadowed spot at the same place and cone, an emissive can above it |

## The balls

| Current engine | Engine 2 |
|---|---|
| Fill: `col x 0.3 + GLASS [236, 232, 226] x 0.7` in four bands of the spot, 88 % opaque, the instrument drawn on it | The glass's far side is the fill (`Surface::back`: that tone, 0.88), lit in four bands of the spot; the fighter's look draws its instrument inside |
| Rim band in `col x 0.9`, 2 px | The fill's outer 9 % of the disc (`RIM_COS`), solid, in the glass tint (`col x 0.9`) |
| Ink outline `[22, 14, 30]`, 1.3 px | An inverted-hull shell at `1.045 R`, unlit ink |
| White specular dot upper left | The spot's specular on the near glass (fresnel 0.95, opacity 0.06), with a faint hammered normal map |
| Contact shadow | Kept |

## Colour and post

| Current engine | Engine 2 |
|---|---|
| sRGB 0..255 everywhere, light multiplying colour directly, no tone map | Colours in linear light, `(c / 255)^2.2` (`stage::linear`). Off-spot set pieces (stands, crowd, skirt) take the current engine's shaded colour divided by the ambient, so the ambient alone reproduces the palette and effect lights still light them. `Post::ring` uses straight light under a soft shoulder (`knee` 0.7) and a 2.2 gamma, so a colour lit to `(c / 255)^2.2` comes out as `c`: the shadows are not crushed as ACES would crush them |
| `grade(contrast 0.18, sat 1.18)`, `vignette(0.45, 0.55, cap 0.5, cy 0.47)`, `top_scrim(0.45, 100, 90)` | The same grade, vignette and scrim in the tone-map shader (`post::grade_cpu`, tested against `bf_picture::post::grade`); the screen chain's own vignette is off |
| Bloom over 225, k 0.22; dither; nearest x3 upscale | Engine 2's bloom (threshold 1.0, 0.1). The pixel look (below) replaces the current dither and x3 upscale |

## Theme

The current picture does not change with a fight's music theme: the ring's palette is fixed in
`bf_picture::scene`, and only the fighters' colours vary. So the TEST DUMMY fight (`fights/engine2/test_dummy.toml`)
already uses the same visual theme as every current fight; its fighters' colours (grey-white and orange) are its own.

## Effects and camera restraint (Aedan: "very subtle for now")

- `bf2_fx::tone::Restraint`, chosen per fighter by `LookFx::tone`, turns the shared presets down for a fighter's
  events: the dummy's hits keep a fifth of the sparks, a third of the glow, 60 % of the life, half-size rings, domes
  and light reach, 55 % marks, 30 % flash and shake, and no lens effects; its K.O. keeps 35 % of the sparks, 30 % of
  the glow, 40 % sizes, 45 % flash and bars, and no lens effects either (no ring push on the screen). The shared
  presets are unchanged for other fighters.
- The dummy's own signature effects are halved in brightness and size.
- The director's moves are scaled by `DirectCfg::intensity` = 0.2.

## Not ported (yet)

- The spot's haze at the top of the frame and the moving spot aim (the current spot follows the action a little).
- The canvas's corner-light tint is weaker than the current one; posts read a little darker where the spot does
  not reach.
- During the K.O. the arena picks up a soft haze from the remaining bloom and grade; it reads restrained but softer
  than the current K.O.

## The pixel look (Aedan's pick, 2026-11-17: "Finer pixels")

The video frame is drawn at 540 x 960 (`renderer::ART_W`, `ART_H`) and each pixel blown up to 2 x 2
(`job2::pixel_scale`, `blow_up`); the UI layer is put over the big frame afterwards, so it keeps its own exact 3x
pixels. The preview draws at 540 x 960 itself. The light is cut into steps inside the scene shaders (`px_at`,
`px_steps` in `common.wgsl`; `lit.wgsl`, `glass.wgsl`): the toon bands, five steps on the set, four on the glass,
with a 4 x 4 ordered dither only where two steps meet (`PX_DITHER` 0.35 of a step), and the glass highlight a hard
dot. It is not a filter after the picture (the reverted E8), so the lighting and effects stay as they are.
Comparison stills of the four options: `/mnt/project-files/ballfight-studio/engine2/pixel-style/`. The
environment can switch it for experiments (`BF2_PX_MODE=0` is the smooth look; see `renderer::px_consts`).
