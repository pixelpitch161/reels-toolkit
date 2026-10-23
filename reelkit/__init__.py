"""reelkit: a tiny low-poly renderer + audio kit for 60 s vertical science/music reels.

Frames are rendered at a low internal resolution (default 360x640), flat-shaded
with PIL polygons in painter's order, post-processed in numpy (shadows, lens blur,
bloom, god rays, grade), then upscaled 3x nearest-neighbour to 1080x1920.

Modules
    config     frame size, fps, Bayer matrix, pixel grids, SAFE (platform safe zone for text/HUD)
    camera     Cam (look-at perspective camera), near-plane clipping
    raster     Frame: colour + float depth buffer, poly/line/disc, glows
    geometry   face/box/prism/lathe/icosa/mound/catenary builders
    lighting   Lights + shade() (keys, fills, rim, sky, point lights)
    shadows    planar shadow masks (directional + point lights), AABB occlusion
    lens       depth of field (octagonal aperture, locked focus), bokeh sprites
    post       bloom, screen-space god rays, grade, vignette, top scrim
    font       5x7 pixel font, outlined text, rich caption lines
    ui         captions with typewriter reveal, HUD panels
    audio      synth helpers, drums, reverb, mastering
    video      parallel frame rendering into a memmap + ffmpeg mux
    sheet      contact sheets for quick visual checks (safe zone drawn on)
    safezone   checks captions/labels/panels sit inside the TikTok / Reels / Shorts safe zone
    loopcheck  checks the loop point is the file boundary (seam, no early reset, no audio click)
    flashcheck photosensitivity (general-flash) test on a finished mp4
    levels     soundtrack engagement check: opening level, build, quiet windows
    backdrops  reusable loop-safe backdrops (VibrantVoid: the vibrant void-of-space sky + stars)
"""
__version__ = '0.1.0'
