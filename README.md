# reels-toolkit

A small Python toolkit (`reelkit`) and playbook for making 60-second vertical
music + science reels in a low-poly, pixel-art-finished style, inspired by (but not
copying) @bit_wonder. Everything is generated in code: the 3D scenes, the simulations,
the lighting, and the soundtrack.

Output: 1080×1920, 30 fps, H.264 + AAC, exactly 60 s. Rendered internally at 360×640
and upscaled 3× nearest-neighbour, which gives crisp pixel edges.

## Quick start

```bash
pip install -e . --break-system-packages      # numpy, scipy, pillow; ffmpeg must be on PATH
cp -r templates/new_video projects/my_video
cd projects/my_video
python audio.py                                 # -> audio.wav
python -m reelkit.sheet render.py test/sheet.png 1 6 12 30 45 58     # quick look
python -m reelkit.video render.py audio.wav my_video.mp4             # full render
```

The full render takes about 10–25 min on 2 cores. It uses every core by default; pass
`--procs N` to change that.

## Layout

| path | what |
|---|---|
| `reelkit/` | shared engine: camera, raster + depth buffer, geometry, lighting, shadows, lens (DOF), post (bloom, god rays, grade), pixel font, captions/HUD, synth audio, parallel video render |
| `templates/new_video/` | a minimal working video using the whole pipeline. Copy it to start. |
| `projects/` | the finished videos' source, for reference and reuse (see each folder's README) |
| `docs/PLAYBOOK.md` | the style rules and the step-by-step process for every video. **Read it first.** |
| `docs/LOG.md` | what each video taught us (feedback received, fixes that worked) |

## Working in parallel

One video per Claude session. Each session clones this repo, copies the template into
`projects/<name>/`, and only changes files inside its own project folder. Changes to
`reelkit/` go in separate, small commits, one at a time, so parallel sessions don't
collide. Commit large outputs (`.mp4`, `.wav`, raw frames, simulation caches) nowhere:
they are git-ignored and regenerated.
