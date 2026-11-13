# BALLFIGHT Studio, milestone 4: the older ten fighters and their themes (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the ten older fighters (ACCORDION, BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA,
VIOLIN) and the twelve themes of the older episodes, each proven exactly the way milestone 3 proved the kit roster, so
that the app offers all 30 fighters and all 26 themes (spec §1, §10, §14 milestone 4).

**Architecture:** Milestone 3's machinery is reused unchanged: the recording is the seam, `bf-sim` gains one module per
weapon and ability kind, `bf-content` one folder per fighter and per theme, `bf-harness` proves each with
`fighter_suite!`, `theme_suite!` and `episode_suite!`, and every reference export goes through `m3refs`. What is new
is the **reference** each port is proven against. The kit has no look, sound or mood module for any of the older ten
or their beds: they exist only inside their episode folders (`render.py`, `styles.py`, `audio.py`), drawn by older
copies of the renderer. So each port first writes the **reference kit module** the kit's own README describes ("copy
its draw functions from its latest episode's `styles.py` and its layers from that episode's `render.py` into the Look
hooks, its part of `audio.py` into Sound, the bed into a mood"), outside `projects/`, installed into the kit at run
time, and proves it against the episode (the **origin gate**: the fighter's layers pixel for pixel and its bus sample
for sample on the episode's own fight). The Rust port is then proven against that module by milestone 3's harness,
bit for bit, as if the module had always been in the kit.

**Tech stack:** as milestones 0 to 3 (`studio/rust-toolchain.toml`, the pinned workspace; Python 3.13.18 with numpy
2.5.3, scipy 1.18.1, Pillow 12.3.0 for the export tool only). No new crates.

**Spec:** `docs/studio/2026-11-10-ballfight-studio-design.md` (§3, §5 to §10, §12 to §14). Section numbers (§) refer
to it. The milestone 3 plan (`2026-11-11-ballfight-studio-plan-m3-kit-roster.md`) is the template this plan copies:
its Templates F, T and P, its Section 3 (lanes), its Global Rules and its rulings M3-R1 to M3-R48 (in the ledger
`.superpowers/sdd/2026-11-11-ballfight-studio-plan-m3-kit-roster/progress.md`) all still hold unless a line here says
otherwise.

**Scope:** spec milestone 4 only. Not here: the music system (milestone 5: every theme here gets its classic
arrangement only, exactly as its episode plays it), packaging (milestone 6). The two retired fighters named exactly
GUITAR and SAX are never offered and never ported; two of the twelve themes come from episodes they fought in, and
those themes are ported without them (Section 5, Table O2; Q2).

**Before Task 1 starts** (all from milestone 3): its Task 59 (starters, `themes_for`) merged; its follow-ups M3-T65
(`Sound::made`) and the open `dsp::convolve_same` order audit (ledger, BASS DRUM entry) merged or explicitly carried
into this plan's Task 6; the stadium_rock drift fix pushed. Tasks 42 to 46 here also wait for milestone 3's Tasks 60
to 63.

---

## 1. What success looks like

Milestone 4 is done when all of these hold, each proven by a named test or command:

1. **All 30 fighters and 26 themes are offered.** `bf_content::available()` returns the 30 fighters of §10 and
   `bf_content::theme_names()` the 26 themes; the Match-up roster shows no "not available yet" chip; the status bar
   reads "30 fighters, 26 themes". Tasks 12, 18, 31, 43.
2. **Each older fighter is its episode's fighter.** Its reference kit module draws its layers and plays its bus as its
   latest episode does on that episode's own fight (Gate O, Template F'); and the Studio port matches that module to
   milestone 3's bars: fights to 1e-6 for at least `PREFIX_FLOOR_S = 10.0` s of fighting in every scenario, every
   event and ability kind at the kit's time, statistics over 2,000 seeds inside the bars, frames, title card and the
   soundtrack's bytes identical. Tasks 7 to 11, 13 to 17.
3. **Each older theme is its episode's bed.** Its reference mood plays the episode's bed buses on the episode's own
   run (Gate O, Template T'); the Studio's classic bed gives the reference mood's 16-bit bytes on its reference
   configs, at a second tempo and key. Tasks 19 to 30.
4. **Each older episode that has no retired fighter is a kit config, proven.** For the ten configs of Table O3 the
   Studio draws the kit's frames (golden set and title card), plays the kit's master, gives every check's verdict and
   numbers, matches the statistics over 4,000 seeds, proves the pair's links, and a Studio-native fight renders to a
   passing mp4 (or meets ruling M3-R47's parity). Tasks 32 to 41.
5. **Every pair of the 30 can start.** `bf_content::starter(a, b)` returns a valid fight file for all 870 ordered
   pairs; `themes_for` filters the 26 themes by the hooks each pair's parts call; six new crossovers equal the kit.
   Task 42.
6. **Nothing old moved.** Every milestone 1 and 3 gate passes unchanged; `studio/replay/hashes.txt`,
   `render_hashes.txt` and every `studio/replay/fighters/*.txt` of the 20 kit fighters are byte-identical;
   `python tools/ballfight_replay_hash.py` prints what it printed before; `git status --short projects reelkit series`
   is empty after every export. Task 46.
7. **The reference set is inside its budget:** at most 147 MB tracked, every cache file pinned, milestone 4's own
   folders inside Section 4's numbers. Tasks 1, 46.

## 2. Global Rules

Milestone 3's Global Rules (its Section 2) apply word for word, with these additions and changes. Every task's
requirements include this section.

- **Nothing in `projects/`, `reelkit/` or `series/` is edited, and nothing is written there.** An episode's scripts
  are run only from a scratch copy under `studio/reference/.cache/m4/origin/<episode>/` (Task 2); the reference kit
  modules are installed into the kit's module names at run time, never as files under `projects/`. After every
  export `git status --short projects reelkit series` is empty (the export checks it and fails otherwise).
- **The reference for an older fighter or theme** is its reference kit module (`tools/studio_export/older/...`), and
  that module is proven against the episode by Gate O before any Rust is written. Where the module and this plan
  disagree, the module wins; where the module and the episode disagree on a gated layer or bus, the episode wins.
- **Which episode.** A fighter's look and sound come from its latest episode folder, with today's roster numbers
  (`projects/ballfight/fighters.py`, never a per-episode pin). Table O1 names the folder for each.
- **Retired fighters.** Only the fighters named exactly GUITAR and SAX are never offered or ported
  (`bf_core::fightfile::RETIRED`); the words may appear in text, names and ids (owner, Nov 12; M3-R42 withdrawn). No
  Studio fight file, recording the Studio writes, starter or crossover has either as a fighter. Python-only origin
  runs of #27 and #29 may run them as data (Q2).
- **Maths.** Rulings M3-R30, R35, R38, R39, R40 and `libm-report.md`'s "Rule for fighter ports" apply to every port,
  module and theme here; no tables of Python's bits.
- **Storage.** Section 4. Every new folder keeps its recordings in the pinned cache (M3-R44, here the default).
- **Lanes.** Six clones as milestone 3's Section 3, with this plan's scratch folder
  `.superpowers/sdd/2026-11-12-ballfight-studio-plan-m4-older-fighters/` for briefs, reports and the ledger.
- **Commits.** Each task's subjects are given in its entry. No model or AI names anywhere; no co-author lines.

## 3. Ownership in this milestone

Milestone 3's three-class rule (its Section 3) holds. The additions:

1. **Owned by one port task:** its reference kit module (`tools/studio_export/older/fighters/<slug>.py` or
   `older/moods/<name>.py`), its export module, its origin data (`origin.json` in its reference folder), plus
   everything Template F, T or P lists. Foundation Tasks 3 and 4 create every Rust file a port owns as a stub first.
2. **Shared, append-only through register tasks:** as milestone 3, plus `older/__init__.py`'s `FIGHTERS`, `MOODS`
   and `EPISODES` tables (filled once by Task 2, never edited by a port).
3. **Shared engine files:** as milestone 3, plus `tools/studio_export/older/{__init__,origin,configs}.py` and
   `older/picture_patches.py` (Task 2 and Task 4 only).

---

## 4. Storage: where reference data lives

**Measured now** (lane/story at 0da5341, every file git tracks under `studio/reference/`, the way `budget` counts):
**139,380,607 bytes (139.38 MB) of the 147 MB cap.** Of that, 133.8 MB is milestone 0 and 1's; the 20 kit fighters
hold 3.16 MB (137 to 227 KB a folder, no recordings: plan B moved them), the 14 themes 1.31 MB (45 to 115 KB, except
`heist` 200.7 KB and `horror_trap` 220.4 KB, which still commit their recording), the 14 pair gates 0.92 MB (62 to
72 KB each).

**What milestone 3 still adds** before Task 1 here: four pair gates (Tasks 55 to 58, about 66 KB each) and Task 59's
crossovers (about 0.1 MB): about +0.37 MB. Task 1 below moves the last two committed theme recordings (-0.26 MB).
**Milestone 4 starts at about 139.5 MB.**

**Decision: no new storage mechanism.** Milestone 3's plan A becomes the only rule: every recording of every new
folder lives in the pinned cache with its SHA-256 committed (M3-R36, R43, R44), and Task 1 makes that the default in
code so no port can forget it. Not Git LFS (milestone 3's reasons stand: local lane remotes, the private repo's quota,
milestone 6's `git pull`).

**What milestone 4 commits, and its budgets** (enforced by `budget` and `bf-testdata`'s `budget` test):

| folder | holds | budget | expected (today's rates) |
|---|---|---:|---:|
| `fighters/<slug>/` x 10 | as milestone 3 (no recording) plus `origin.json` (Gate O's counts and hashes, at most 20 KB) | 220 KB | about 165 KB |
| `themes/<name>/` x 12 | as milestone 3 (no recording) plus `origin.json` | 120 KB | about 85 KB |
| `episodes/<pair>/` x 10 | as milestone 3 | 100 KB | about 68 KB |
| `older/` (Task 4's shared vectors, Task 2's origin self-proof) | top-level manifest | 400 KB | about 300 KB |
| `episodes/crossovers/` (Task 42's six) | as milestone 3 | inside the episodes row | about 60 KB |

Worst case (every folder at its cap): 2.20 + 1.44 + 1.00 + 0.40 + 0.06 = **5.10 MB, ending at about 144.6 MB.**
Today's rates: 1.65 + 1.02 + 0.68 + 0.30 + 0.06 = **3.7 MB, ending at about 143.2 MB.** Both leave at least 2.4 MB.

**Tripwire.** `budget` prints a warning when the tracked total passes **145.5 MB**. At that point, and only then, the
controller applies reserve C1 of milestone 3's `storage-plan.md` (the 20 kit fighters' committed RMS profiles in
`sound.json` / `voice.json`, about 1.5 MB, to the pinned cache with their SHA-256 kept; the always-run sound gates then
compare the master's SHA-256 and the counts). Reserve C2 (milestone 1's 65 MB of stems, premasters and DSP vectors)
stays out of this milestone.

**What is never committed:** episode scratch copies and their outputs (`.cache/m4/origin/`), the origin gate's frames,
layer images and stems (cached and pinned only when a Rust gate reads them; otherwise only their hashes go into
`origin.json`), every recording. The ten new meshes (`bf-content/data/models/*.json`) are compiled content outside the
budget, as in milestone 3 (20 meshes are 7.52 MB today; Task 45 reports the new total).

---

## 5. Survey

Measured in this clone: `projects/ballfight/fight.py` (6,242 lines), `fighters.py` (576), `title.py` (902), the
episode folders (only the delivered files, never the `*_backup.py` or `v1/`, `v2/`, `v3/` snapshots), `kit/` and
`projects/which_instrument_wins/pp_bridge.py`. Every older weapon and ability kind is already in `fight.py` and every
older fighter is already in `fighters.py`: the fight engine needs no Python port, only the Rust one, and Template F's
fight half runs unchanged.

### Table O1: the ten fighters

Kinds marked *ported* exist in the Studio after milestone 3. "Look" is the episode's `styles.py` functions and the
`render.py` layers; "Part" the episode's `audio.py` functions; "Voice" the `pp_bridge` entry (line) and module.

- **BAGPIPES** Episode: `ballfight_bagpipes_vs_accordion` (#30 v3). Roster 150-162. Weapon `inflate`: `inflate_setup`
  1763, `inflate_start` 1777, `inflate_step` 1792, `inflate_clash` 1839-1867, its branch of `grid16_fire` 1487-1507,
  `run()` 5761-5765; **per-ball radius and mass** (`rad`, `mass`: `move` 3179, 3209-3216, `clash_hooks` 3233; every
  kind that reads `getattr(O, 'rad', R)`), record key `RAD`. Abilities: `shockwave`, `haste`, `wave` (all ported).
  Look: `tartan_rgb` 83, `draw_trail` 98, `tartan_ring` 286, render `_cycles` 87, `cycle_at` 107, `fighter` 215
  (the swell drawn as a bigger ball). Title: `title.py` `_tartan`. Part: `chanter` 93, `drone_layer` 134, `deflate`
  244, `squeal` 257, `tear` 272 (its own synth; `pp_bridge` 41 `bagpipes60_lowpoly`, 845 lines, gives the model).
  **Size L** (a per-ball size every kit kind's contact must respect).
- **ACCORDION** Episode: #30 v3. Roster 163-176. Weapon `bite`: `bite_fire` 1868, `bite_snap` 1880-1904, the bite branch
  of `grid16_fire`. Abilities: `haste`, `shield` (ported), `clamp` (`clamp_start` 1905, `clamps_step` 1915-1926,
  trigger 2874-2877: lands unless shielded). Look: `bite_open` 113, `draw_jaws` 132, `clamp_state` 176, `draw_clamp`
  203, `collar_polys` 246, `draw_squeezebox` 260; render `_bites` 123, `bites_at` 141, `_clamps` 150. Title `_pleats`.
  Part: `reeds` 106, `acc_chord` 292 (own synth; model `accordion60_lowpoly`, 984 lines). **Size M.**
- **BARI SAX** Episode: `ballfight_bassoon_vs_barisax` (#39 v4, Oct 27: the roster's current BARI SAX with `steer`;
  #40 v2 pins `steer` off and is older). Roster 416-438. Weapon `wrecking`: `wreck_setup` 3594, `_wreck_aim` 3601
  (aims where a hop lands, round an anchored whirl), `wreck_tick` 3666, `_sails_over` 3685, `wreck_step` 3697-3786
  (airborne, out over the ropes, `steer`). Abilities: `shockwave` (ported), `offbeat` (SKANK, trigger 2911, its cycles
  inside `wreck_tick`), `snap` (CABLE SNAP: `start_snap` 3787, `snap_step` 3807-3856). Records `WRECK`, `SNAP`. Look:
  `cable` 257, `hook_block` 271, `hook_shape` 285, `harness` 301, `swing_trail` 327, `snap_target` 343, `honk_light`
  178, `draw_honk_front` 198, `rope_wave` 211; render `_cycles` 197, `cycle_at` 234, `wreck_pose` 242, `snap_at` 259,
  `air_pose` 270, `bari_over` 469, `_rope_wobbles` 486, `rope_amp` 512, `crane_state` 791, `draw_crane` 886,
  `draw_swing_fx` 905. Title `_barisax`. Part: `bari` 108, `gliss` 117, `bend` 127, `doppler` 136, `rope_twang` 149,
  `winch` 164 (`pp_bridge` 56 `barisax_lowpoly`, 411 lines). **Size L** (the largest kind; two links).
- **SYNTH** Episode: `ballfight_turntables_vs_synth` (#31 v3). Roster 324-340. Weapon `tether`: `_tether` 1572,
  `tether_beat` 1578, `tether_16` 1594, `tether_step` 1608-1660 (the leash: bounces off unless flung harder than
  `snap` x cables). Abilities: `haste`, `wave` (ported), `lash` (`start_lash` 1661, `lash_angle` 1682, `lash_step`
  1687-1712). Look: `cable_points` 321, `jack_quad` 339, `saw_arm_points` 350, `saw_points` 108, `draw_saw` 127,
  `wave_shape` 311; render `cable_ends` 324, `live_cables` 329, `draw_cables` 1006, `draw_jacks` 1105. Title: the
  episode's `_circuit` (render 92, registered into `TT.PATTERNS`). Part: `arp_note` 92, `arp_lv` 184, `jack_click`
  202, `zip_out` 211, `cable_snap` 220, `supersaw_sweep` 292, `noise_blast` 301 (`pp_bridge` 50 `synth60_lowpoly`,
  1,042 lines). **Size L.**
- **TURNTABLES** Episode: #31 v3. Roster 313-323. Weapon `scratch_floor`: `scratch_beat` 1713, `scratch_step`
  1731-1762. Abilities: `haste`, `slam` (ported), `rewind` (`_rewind_ab` 1927, `_rewind_then` 1930, `rewind_ready`
  1940 (its gate at 3035), `start_rewind` 1946, `rewind_step` 1963-1998; **a rewinding ball is intangible**:
  `rw_left` in `damage` 325-326, `move`, and every kind that tests it). Look: `record_albedo` 238, `tonearm` 286,
  `drop_height` 135, `drop_polys` 150, `rewind_glyph` 191, `vhs` 161; render `scratch_q` 148, `floor_angle` 185 (the
  floor turns), `rewinding` 221, `arm_state` 273, `draw_drop` 1092, `draw_tonearm` 1112, `post_fx` 1404. Title:
  `_grooves` (render 81). Part: `scratch` 135, `floor_scratch` 166, `rewind_whirr` 317, `stab` 97 / `_stab_src` 119
  (`pp_bridge` 49 `turntables60_lowpoly`, 1,207 lines; MIDI below 12 = scratches). **Size L** (an intangible state
  every kind must respect; the floor's rotation is new to the renderer).
- **VIOLA** Episode: `ballfight_viola_vs_violin` (#42). Roster 439-454. Weapon `rests`: `rests_setup` 5021,
  `fire_note` 5034, `rests_tick` 5064, `_crush` 5087, `notes_step` 5137-5181 (a rolling note flattens projectiles),
  the resting `guard` in `damage` 342-343; record key `REST`. Abilities: `double_stop` 5108, `melody` (trigger
  2923-2927, `rests_tick` 5073-5076), `joke` 5118-5136 (stuns the enemy). Look: `note_axes` 59, `note_centre` 75,
  `note_quads` 81, `draw_note` 110, `note_shadow` 140, `glyph` 155, `stamp_glyph` 160; render `_notes_build` 163,
  `notes_at` 197, `rest_state` 227, `stun_k` 235, `laugh_k` 243, `melody_k` 252, `render_padded` 511, `draw_rests`
  1068. Title `_viola`. Part: `viola` 116, `rimshot` 215, `ha` 234, `crowd_laugh` 249 (`pp_bridge` 57
  `viola_lowpoly`, 326 lines, size x1.12). **Size L.**
- **UKULELE** Episode: `ballfight_ukulele_vs_acoustic` (#33 v3). Roster 259-270. Weapon `surf`: `surf_dist` 502,
  `fire_surf` 506-527, the `surfs` part of `surf_rake_step` 588-609, on the `dash` / `cone` / `surf` / `rake` 16th
  counter (5751, shared with ACOUSTIC's `rake`). Abilities: `shockwave`, `haste` (ported), `spout` (`ab_x33` 578-583,
  `surf_rake_step` 671-699, the path in `X33_SPOUT` 6221). Look: `wave_quads` 263, `draw_wave` 289,
  `wave_shadow_quads` 322, `spout_radius` 424, `draw_spout` 430, `WATER` 255; render `_surfs` 89, `fight_surf_dist`
  114, `surf_height` 150, `surf_front` 160, `waves_at` 168, `spout_at` 191. Title `_surf`. Part: `strum` 89,
  `uke_strings` 113, `ks` 125, `uke_part` 363-396, `wave_crash` 336, `water_wind` 347 (`pp_bridge` 42
  `ukulele60_lowpoly`, 717 lines). **Size L** (shares a step and a counter with a merged port).
- **VIOLIN** Episode: #42 (as #28 built it; #42's `styles.py` imports #28's). Roster 123-134. Weapon `blade`, the
  spinning branch of `blade_step` 780-825 with `spin_grow` / `spin_max` (`blade` thrust is ported). Abilities: `arc`,
  `haste`, `wave` (all ported). Look: #28 `bow_geom` 29, `draw_bow` 47, `bow_trail_polys` 61, `dart_quads` 134,
  `draw_dart` 143; #42 render's VIOLIN layers. Title `_violin`. Part: #42 `bow_note` 110, `pizz` 150, `squeal` 162,
  `scratch` 142 (`pp_bridge` 28 `violin_lowpoly`, 506 lines; CELLO's port already holds `violin.bowed` and the violin
  module's noise stream). **Size M.**
- **HARP** Episode: `ballfight_organ_vs_harp` (#35 v4). Roster 239-258. Weapon `trap`: `lay_string` 1363-1392, the
  strings' half of `hazards_step` 1417-1434, `hazard_beat` 1324-1332 (shared with ORGAN's `line`). Abilities: `arc`,
  `shield`, `wave` (ported). Look: `string_state` 134, `string_points` 147, `string_twang` 171, `draw_string` 180,
  `dart_quads` 195, `draw_dart` 204, `wing_polys` 297, `draw_wings` 317, `draw_halo` 325; render `strings_at` 809,
  `solo_strings` 97. Title `_harp_strings`. Part: `harp` 90 and its branches of `main` 310-549 (`pp_bridge` 30
  `harp60_lowpoly`, 448 lines). **Size M.**
- **DRUMS** Episode: `ballfight_drums_vs_violin` (#28, predates the VS card). Roster 112-122. Weapon `pulse`:
  `fire_pulse` 411-424, the `pulse` / `volley on16` counter 5946-5963 (haste pulses hit for `sub_dmg`; `clears`
  swats projectiles). Abilities: `haste`, `shield`, `slam` (ported). Look: `pulse_state` 99, `annulus_polys` 82,
  `draw_drumhead` 112, `cymbal_polys` 155, `draw_cymbal` 175, `beater_height` 208, `draw_beater` 233; render
  `_pulses` 77, `solo_pulses` 93. Title: no pattern found (plain panel unless the port finds one). Part: `drum` 85,
  `sticks` 133, `roll` 181 (`pp_bridge` 39 `drums60_lowpoly`, 445 lines). **Size M.**

Kinds that stay `SLOT` comments (no fighter offered by the Studio uses them; they belong to frozen versions or to the
retired fighters): `volley` (except the `on16` counter DRUMS shares), `orbit`, the `beam` weapon, `flail`, `aura`,
`bellows`, `dash`, `cone`, `pack`, `rattle`, `quake`, `charge`.

### Table O2: the twelve themes

"Bed" is the episode's `audio.py` bed functions; the reference mood moves exactly those (and the sections of `main()`
that belong to the bed, not to a fighter) into a kit `Bed(M)` class.

- **`drift_phonk`** Episode: `projects/ballfight/` (#27; its fighters are GUITAR and PIANO v1, neither ported). Bed:
  `bed` 199-235, `cowbell` 111, `b808` 119, `kick` 132, `roll` 137, `riser` 150, `roar` 169, `shimmer` 187. 129.84
  BPM, E minor with the flat 2nd. Plays no fighter voice. Reference configs: two stand-in kit pairs (Q2). **Size S.**
- **`balkan_fire`** Episode: #28. Bed: `upright` 143, `reed` 156 (the musette), `cimbalom` 171, `roll` 181, `riser`
  198, `bed` 240-272; no drums of its own (the DRUMS fighter is the kit). 141.64 BPM, D harmonic minor. Reference:
  `drums_vs_violin`. **Size M.**
- **`noir_jazz`** Episode: `ballfight_trumpet_vs_sax` (#29; SAX is retired). Bed: `upright` 136, `ride` 152,
  `brush_tap` 166, `swirl` 173, `stick_snare` 182, `piano_chord` 186 (plays the PIANO voice, ported), `dark_pad` 191,
  `roll` 204, `walk` 264, `bed` 277-313. 133.77 BPM, C minor, loop i - bVI7 - iv - V7. Reference configs: two
  stand-in kit pairs (Q2). **Size M.**
- **`sea_shanty`** Episode: #30. Bed: `stomp` 151, `gang_clap` 161, `frame_drum` 171, `choir` 185, `bass_voice` 205,
  `riser` 217, `bed` 305-328. 121.97 BPM, A minor. Reference: `bagpipes_vs_accordion`. **Size M.**
- **`dark_breakbeat`** Episode: #31. Bed: `kick` 231, `snare` 238, `crunch` 247, `reese` 254, `cold_pad` 271, `wobble`
  277, `sub_drop` 309, `break_bar` 371, `bed` 388-401. 137.70 BPM, F minor. Reference: `turntables_vs_synth`.
  **Size M.**
- **`villain_march`** Episode: `ballfight_tuba_vs_flute` (#32 v3). Bed: `sub_drone` 145, `brass` 157, `horn_pad` 174,
  `music_box` 185, `bass_drum` 195, `march_snare` 199, `timpani` 208, `roll` 218, `bed` 262-291. 125.90 BPM, C minor.
  Reference: `tuba_vs_flute` (both kit fighters). **Size M.**
- **`spaghetti_western`** Episode: #33 v3. Bed: `twang` 142, `hoof` 160, `thump` 170, `whip` 175, `bell` 190, `choir`
  203, `tremolo_strings` 213, `whistle` 223, `whistle_phrase` 324, `scrape` 266, `bed` 285-323. 129.84 BPM, A minor,
  Am - G - F - E. Reference: `ukulele_vs_acoustic`. **Size M.**
- **`gothic_horror`** Episode: #35 v4. Bed: `choir` 115, `bell` 135, `tick` 149, `heart` 158, `timp` 163, `cello` 174,
  `drone` 182, `roll` 191, `bed` 258-309. 125.90 BPM, D minor. Reference: `organ_vs_harp`. **Size M.**
- **`brass_house`** Episode: `ballfight_trombone_vs_tuba` (#36; milestone 3's `trombone_vs_tuba` config used the big
  top as a stand-in). Bed: `kick` 212, `hat` 218, `ride` 225, `shaker` 232, `stab` 238, `pump` 253, `pad_bar` 265,
  `bed` 271-297; no brass and no bass of its own. 125.90 BPM. Reference: `trombone_vs_tuba_brass_house`. **Size M.**
- **`ska_punk`** Episode: #39 v4. Bed: `skank` 214, `organ` 234, `ride` 252, `tom` 271, `thunder` 281, `band_bar` 339,
  `band_hit` 374. 169.18 BPM (the fastest), E minor. Reference: `bassoon_vs_barisax`. **Size M.**
- **`industrial`** Episode: `ballfight_barisax_vs_bassclarinet` (#40 v2). Bed: `anvil` 188, `backbeat` 201, `pipe`
  213, `tick` 224, `grind` 239, `sheet` 259, `hiss` 273, `wrench` 288, `band_bar` 389, `band_hit` 427,
  `busy_windows` 441 (reads the fight's busy stretches). 125.90 BPM, D minor. Reference: `barisax_vs_bassclarinet`.
  **Size L** (reads the recording).
- **`orchestral_drill`** Episode: #42. Bed: `tom` 206, `sub808` 275, `hat` 288, `clap` 292, `rim` 298, `cello` 305
  (plays the CELLO voice, ported), `contrabass` 309, `bed` 327-371. 141.64 BPM, C minor. Reference: `viola_vs_violin`.
  **Size M.**

### Table O3: the ten configs a pair gate proves

Each is written as a kit config (`tools/studio_export/older/configs/<pair>/episode.py`, Task 5) with today's roster,
the series-standard beat sheet (VS card on bar 0, FIGHT! on bar 2, the kit's tail), the episode's theme, tempo, key,
scale, loop, lines, solos, roles, damage scale and handicap, and the episode's seed. Its recording is the kit's run of
that config (which is the episode's fight only where the episode already used the 2-bar start and today's numbers).

| Task | Config | Episode | Theme | Fighters' tasks | What it adds |
|---|---|---|---|---|---|
| 32 | `drums_vs_violin` | #28 | balkan fire | 17, 14 | a pulse that swats darts; the bow against the drumhead |
| 33 | `bagpipes_vs_accordion` | #30 | sea shanty | 7, 16 | an inflated bag against the bite; a clamp on a swollen ball |
| 34 | `turntables_vs_synth` | #31 | dark breakbeat | 8, 10 | the leash against the floor's fling; a rewind out of a leash |
| 35 | `tuba_vs_flute` | #32 | villain march | kit | the march under two kit fighters |
| 36 | `ukulele_vs_acoustic` | #33 | spaghetti western | 13, kit | surf and rake on one shared counter; the chord solo (M3-T64) |
| 37 | `organ_vs_harp` | #35 | gothic horror | 15, kit | strings and pipe rows in one `hazards_step` |
| 38 | `trombone_vs_tuba_brass_house` | #36 | brass house | kit | the episode's own music for a milestone 3 pair |
| 39 | `bassoon_vs_barisax` | #39 v4 | ska punk | kit, 9 | a hop sails over the wrecking ball (`_sails_over`) |
| 40 | `barisax_vs_bassclarinet` | #40 v2 | industrial | 9, kit | the swing aimed round an anchored whirl; LIGHTS OUT over the crane |
| 41 | `viola_vs_violin` | #42 | orchestral drill | 11, 14 | a rolling note flattens pizzicato darts; VIOLA JOKE's stun |

Not configs here: #27 (GUITAR) and #29 (SAX). #40's own sim pins `steer` off; its config here runs today's BARI SAX
(Q3), so its fight is not #40's.

### Table O4: links between fighters

- **A per-ball radius and mass** (`inflate`: `move` 3179-3216, `clash_hooks` 3233, every kind reading `rad`).
  Changes: BAGPIPES with everyone. Proven at: Task 7 (scenarios against all 20 kit fighters), 33.
- **A rewinding ball is intangible** (`rw_left`: `damage` 325-326, `move`, `notes_step`, `wreck_step`, `tether_step`).
  Changes: TURNTABLES with everyone. Proven at: Task 8 (scenarios against all 20), 34.
- **A hop sails over a swing; a swing aims at a hop's landing or round a whirl** (`_wreck_aim` 3601, `_sails_over`
  3685). Changes: BARI SAX with BASSOON, BASS CLARINET. Proven at: Tasks 9, 39, 40.
- **The leash holds unless the knock is harder than `snap` x cables** (`tether_step` 1608-1660). Changes: SYNTH with any
  big knock (TUBA's spit, HORN's FINAL FLICK, slams). Proven at: Task 10 (scenarios against TUBA, HORN), 34.
- **A resting viola takes `guard` x damage; a rolling note flattens projectiles; VIOLA JOKE stuns** (`damage` 342-343,
  `notes_step` 5137-5181, `joke` 5118-5136). Changes: VIOLA with VIOLIN, HARP (darts), PIANO (keys), everyone (stun).
  Proven at: Task 11 (scenarios against PIANO), 41.
- **Surf, spout and rake share `surf_rake_step` and the 16th counter** (585-700, 5751). Changes: UKULELE with ACOUSTIC.
  Proven at: Tasks 13, 36.
- **Strings and pipe rows share `hazards_step`; the owner crosses its own strings freely** (1393-1434). Changes: HARP
  with ORGAN. Proven at: Tasks 15, 37.
- **A clamp lands unless shielded** (`clamps_step` 1915-1926). Changes: ACCORDION with shield owners (DRUMS, HARP,
  TUBA, HORN, ORGAN). Proven at: Task 16 (scenario against TUBA), 33.
- **A pulse clears projectiles** (`clears`, `fire_pulse` 411-424). Changes: DRUMS with VIOLIN, HARP (darts), PIANO
  (keys). Proven at: Tasks 17, 32.
- **IDENTITY CRISIS copies a generic ability** (`COPYABLE` 2802: shockwave, wave, shield, haste, arc, slam) **and draws
  it through the enemy's look** (`setup_as`). Changes: EUPHONIUM with each older fighter. Proven at: each fighter
  task's copy sweep (fight parity and no-panic drawing, as M3-R3).

**Totals.** About 1,600 lines of `fight.py` for the older kinds, about 4,500 lines of episode look code (`styles.py`
plus the fighters' parts of `render.py`), about 2,500 of fighters' parts and 2,000 of beds in `audio.py`, and 6,900
lines of voice modules (most of each is model and video code). Sizes: as milestone 3 (S, M, L).

---

## 6. Review Focus

Failure modes the spec implies but states no test for, most likely first. Each has a test in the task named.

1. **An older state meeting a kit kind written for "two balls of radius R, always solid".** An inflated bag (per-ball
   `rad`, `mass`) or a rewinding deck (`rw_left`) against a kind whose Rust port assumed `R + R` or a tangible enemy.
   Python reads the per-ball value at those sites; the Rust must too. Test: BAGPIPES and TURNTABLES each fight all 20
   kit fighters (3 seeds each side, prefix agreement to the floor, every kit kind's contact event seen). Tasks 3, 7, 8.
2. **An episode script writing into `projects/`** (its `check/` caches, `audio.wav`, sprites). Test: the origin runner
   refuses a source folder that is not the scratch copy, and every export ends with an empty
   `git status --short projects reelkit series`. Task 2.
3. **A theme whose bed leans on its episode's fighters** (balkan fire has no drums because DRUMS is the kit; industrial
   reads the bari's busy stretches; brass house has no bass because the TUBA is the bass) **played under a pair
   without them.** The kit plays its classic bed as written; the Studio must give the reference mood's bytes, never
   panic, and say on the theme card what it leaves out. Test: each theme's bed alone under a stand-in pair of other
   fighters equals the reference mood's master. Template T', Tasks 19 to 30.
4. **A retired fighter reaching a Studio file** through a theme whose episode it fought in (drift phonk, noir jazz).
   Test: those themes' Rust gates use stand-in recordings only; milestone 3's `retired.rs` runs every pair of the 30
   and every one of the 26 themes. Tasks 19, 21, 43.
5. **EUPHONIUM copying an older fighter's ability** and drawing it through that fighter's look made for the other side
   (`setup_as`). Test: each fighter task's copy sweep (EUPHONIUM against X, 3 seeds each side: fight parity, every
   copied kind drawn without a panic). Template F'.
6. **An old timeline in a fight file** (FIGHT! on bar 1, no VS card, an old tail, a per-episode roster pin). The
   Studio has one beat sheet and one roster; `validate` refuses a fight file that asks for anything else, with a
   one-line message naming the field. Task 5.
7. **A missing or stale cache for an older folder** fails with the command to run, never passes. Inherited from
   milestone 3's harness; each task's report shows one such failure by name. Template F'.

---

## 7. Order of work

Phase 0 is the six foundation tasks. After it the ports run in up to six lanes.

- **1 Storage for milestone 4** Can run beside: 2, 3; Notes: Python lane, small.
- **2 Reference kit modules and the origin gate** Can run beside: 1, 3; Notes: the largest foundation; proven on two
  kit ports against their episodes.
- **3 Engine sockets for the older kinds** Can run beside: 1, 2, 5; Notes: pre-flight scan after it.
- **4 Shared helpers, picture opt-ins, stubs** Needs: 2 (the patch module), 3 (the kinds); Can run beside: 5, 6.
- **5 The older configs and the fight file** Needs: 2; Can run beside: 3, 4, 6.
- **6 `np.convolve`'s order** Needs: nothing; Notes: skipped when milestone 3 merged it.
- **7 to 11 batch A** Needs: 1 to 6; Can run beside: each other, themes.
- **12 Register A** Needs: 7 to 11; Notes: controller.
- **13 to 17 batch B** Needs: 1 to 6; 13 needs nothing more (ACOUSTIC is merged); Can run beside: each other, themes.
- **18 Register B** Needs: 13 to 17; Notes: controller.
- **19 to 30 themes** Needs: 1, 2, 5 (their reference configs), 6; Can run beside: any port. They use the R25 stand-in
  for an unported fighter.
- **31 Register themes** Needs: 19 to 30; Notes: controller.
- **32 to 41 pair gates** Needs: both fighters' tasks, the theme's task and the register tasks that offered them.
- **42 Starters for 30, crossovers** Needs: 12, 18, 31, 32 to 41, milestone 3's Task 59.
- **43 The app with 30 fighters** Needs: 42, milestone 3's Task 60.
- **44 Full renders** Needs: 42; Can run beside: 43.
- **45 Measurements** Needs: 43, 44; Notes: other lanes idle.
- **46 Docs and the closing run** Needs: 45.

**Suggested lane fill** (46 tasks; each line one round of up to six lanes):

1. 1, 2, 3, 6; then 4 and 5 when 2 and 3 merge.
2. 7, 8, 9, 10, 11 (batch A) and theme 19.
3. 13, 14, 15, 16, 17 (batch B) and theme 20; 12 when batch A merges.
4. 21 to 26; 18.
5. 27 to 30; 31; the first pair gates whose needs are met.
6. The remaining pair gates.
7. 42; then 43 and 44; then 45 and 46.

**Pre-flight scan** (after Task 3, before batch A): the controller compares every new socket with the `fight.py`
lines Table O1 names, as milestone 3 did, and lists any disagreement in the ledger. Batch A starts when the list is
empty. **Gate O checkpoint**: in every port, the reference module and its `origin.json` are reviewed by the
controller before the Rust work of that task starts (a wrong reference would make every later gate prove the wrong
thing).

---

# Foundations

### Task 1: Storage for milestone 4

**Files:**
- Modify: `tools/studio_export/m3refs.py` (the defaults), `tools/studio_export/README.md`,
  `studio/crates/bf-testdata/src/budget.rs`, `studio/crates/bf-testdata/tests/budget.rs`
- Remake (controller-authorised, M3-R1): `studio/reference/themes/heist/`, `studio/reference/themes/horror_trap/`

**Interfaces:**
- Produces: `SPEC.left_in_cache`, `SPEC.recordings_in_cache` and `SPEC.recording_in_cache` default to `True`;
  `m3refs.COMMITTED_RECORDINGS` (the named list of folders still allowed a committed `.bfrec`: milestone 1's
  `fights/`, nothing else after the remake); `budget` rows for `older/` (400 KB) and an `origin.json` limit (20 KB)
  inside each fighter and theme folder; `budget` prints `warning: the reference set is <n> MB, past the 145.5 MB
  tripwire (plan m4 Section 4)` above 145.5 MB and still fails only above 147 MB.
  ```rust
  // bf_testdata::budget
  pub const TRIPWIRE: u64 = 145_500_000;
  pub fn budget_report() -> Vec<Budget>;   // as milestone 3, with the new rows
  ```

- [ ] **Step 1: Write the failing tests.** `m3refs_checks.py`: a fighter, theme and pair SPEC with no storage key
  writes its recording to the cache; a committed `.bfrec` in a fighter, theme or episode folder fails `budget` naming
  the file; an `origin.json` of 20,001 bytes fails; 145.6 MB prints the warning and passes. Rust:
  `budget.rs::the_set_is_inside_its_budget` with the new rows; `budget.rs::the_tripwire_warns`.
- [ ] **Step 2:** Run: FAIL.
- [ ] **Step 3: Implement**, then `remake themes/heist themes/horror_trap` with `recordings_in_cache` (each folder's
  other committed files byte-identical, two pins added).
- [ ] **Step 4:** `verify`, `budget` (expect about 139.1 MB plus whatever milestone 3 merged since), the two themes'
  suites by name with the cache present: PASS. `cargo test -p bf-testdata`: PASS.
- [ ] **Step 5:** Commit `reference: recordings in the cache by default; milestone 4 budgets`.

### Task 2: Reference kit modules and the origin gate

**Files:**
- Create: `tools/studio_export/older/__init__.py` (the tables and `install`), `older/origin.py` (the episode runner and
  the comparisons), `older/fighters/__init__.py`, `older/moods/__init__.py`, `older/configs/__init__.py`,
  `older/picture_patches.py` (empty registry; Task 4 fills what the looks need),
  `tools/studio_export/unit_checks/origin_checks.py`, `tools/studio_export/origin_selfproof.py`,
  `studio/reference/older/selfproof/` (hashes), `studio/docs/porting-an-older-fighter.md`
- Modify: `tools/studio_export/m3refs.py` (call `older.install()` in every child process before the kit is imported;
  `fighter_refs`, `theme_refs` and `episode_refs` read an `origin` SPEC key), `tools/studio_export/m3parts.py` (the
  same in `_child`), `tools/studio_export/README.md`

**Interfaces:**
- Produces, Python:
  ```python
  # older/__init__.py
  FIGHTERS = {'BAGPIPES': 'ballfight_bagpipes_vs_accordion', ...}    # Table O1's latest episode per fighter
  MOODS = {'balkan_fire': 'ballfight_drums_vs_violin', 'drift_phonk': 'ballfight', ...}   # Table O2
  def install():  # idempotent: for every older/fighters/<slug>.py and older/moods/<name>.py that sets PORTED = True,
                  # put it in sys.modules under the name the kit imports (kit.fighters.module(name) and
                  # music.py's moods.<MOOD>), and apply older/picture_patches.py's registered patches; writes nothing
  # older/origin.py
  def scratch(episode: str) -> pathlib.Path   # a copy of projects/<episode>/'s delivered sources under
                                              # studio/reference/.cache/m4/origin/<episode>/, run there; refuses any
                                              # other folder as a working directory
  def episode_frames(episode, frames, layers_off=()) -> dict   # the episode's own render.py, frame -> RGB array
  def episode_stems(episode) -> dict                            # the episode's own audio.py, bus -> float64 stem
  def fighter_origin(spec) -> dict    # Gate O for a fighter: writes origin.json (below) or SystemExit naming the frame,
                                      # the layer, the first differing pixel, or the bus and the first sample
  def mood_origin(spec) -> dict       # Gate O for a theme
  ```
  A port's `SPEC` gains `origin=dict(episode=..., seed=..., frames=<at least 24>, layers=[...], buses=[...],
  window=('fight', 'shatter'))`.
- **What Gate O compares** (the kit's own porting method, made exact):
  - *Fight:* the kit's run of the episode's own config (its `sim.py` rules, built in the scratch copy) and the
    replay guard's line for that episode are equal (`tools/ballfight_replay_hash.py`'s child command for it).
  - *Layers:* for every chosen frame inside the window (FIGHT! to the K.O. shatter: the card and the tail are the
    series standard in the kit, not the episode's), the episode's renderer with the fighter's layers on and off, and
    the kit's renderer with the reference module and with `base.Look`; the two layer contributions (on minus off,
    per pixel, in `int16`) are identical, and the module's per-frame state (what its `state()` gives the ring) equals
    the episode's values at the same frame. At least 24 frames: each ability 0.2 s and 0.6 s after its first use, a
    weapon hit, HP below 25, its shards up to the shatter, plus a spread.
  - *Sound:* the module's `Sound` bus on the episode's run equals the episode's own stem for that fighter over the
    window, sample for sample in float64 (bar: -120 dBFS, documented exceptions to -80 dBFS, reviewed by the
    controller; the kit's ports reported "bit-identical" or "1e-16").
  - *Bed (themes):* the mood's buses with both fighters' parts silent equal the episode's bed stems over the whole
    61 s (the bed has no shatter window); where the episode mixes a bed bus with a fighter's, the episode is run with
    that fighter's placements removed at run time, and the report names the patch.
  - `origin.json`: the episode, its commit, the frames and layers compared with their SHA-256, the buses with their
    peak difference in dBFS, every exception with its reason. Images and stems go to the cache.
- **The self-proof.** `origin_selfproof.py` runs Gate O for two modules the kit already holds, against the episodes
  they were ported from: ORGAN (`kit/fighters/organ.py` against #35 v4, the README's "frames pixel-identical in a
  matched harness, organ buses bit-identical") and the `brass_trailer` mood against #34's audio. Both must pass as
  written; that is the proof that Gate O can pass for a faithful port and fails for an unfaithful one.

- [ ] **Step 1: Write the failing tests.** `origin_checks.py`: `scratch` copies sources only and refuses to run in
  `projects/`; an episode run leaves `git status --short projects reelkit series` empty; `install` twice changes
  nothing; a module with `PORTED = False` is not installed (the kit draws the bare ball); a one-pixel change in a
  layer and a one-sample change in a bus each fail Gate O naming the frame or the sample (mutations of the ORGAN
  module at run time).
- [ ] **Step 2:** Run: FAIL.
- [ ] **Step 3: Implement**, then `porting-an-older-fighter.md`: milestone 3's `porting-a-fighter.md` plus Step 0 (write
  the module from the episode, run Gate O, read a failing layer with the stage dumps, what goes in `origin.json`).
- [ ] **Step 4:** `python -m tools.studio_export origin_selfproof`: ORGAN and brass trailer PASS, the mutations FAIL;
  `verify`; `budget`; `git status --short projects reelkit series` empty.
- [ ] **Step 5:** Commit `tools/studio_export: reference kit modules for the older fighters, the origin gate`.

### Task 3: Engine sockets for the older kinds

**Files:**
- Modify: `studio/crates/bf-core/src/roster/{mod.rs,kinds/mod.rs}`, `studio/crates/bf-sim/src/{fight,schedule,record,
  ball,lib}.rs`, `studio/crates/bf-sim/src/kinds/{mod,blade,line,rake}.rs` (the `SLOT` lines of Table O1 become calls),
  `studio/crates/bf-sim/src/abilities/{mod,x33}.rs`, `studio/docs/recording-format.md` (one marked section per kind),
  `studio/crates/bf-sim/tests/sockets_order.txt`
- Create (stubs, each later owned by one port): `bf-core/src/roster/kinds/{pulse,inflate,bite,trap,surf,
  scratch_floor,tether,wrecking,rests}.rs`, `roster/abilities/{clamp,rewind,lash,spout,offbeat,snap,double_stop,
  melody,joke}.rs`, `bf-sim/src/kinds/{pulse,inflate,bite,trap,surf,scratch_floor,tether,wrecking,rests}.rs`,
  `bf-sim/src/abilities/{clamp,rewind,lash,snap,joke}.rs` (`spout` lives in `x33.rs`'s branch, owned by UKULELE;
  `offbeat` in `wrecking`; `double_stop` and `melody` in `rests`); `bf-sim/tests/sockets_older.rs`

**Interfaces:**
- Produces:
  ```rust
  // bf_core::roster: WeaponCfg gains Pulse, Inflate, Bite, Trap, Surf, ScratchFloor, Tether, Wrecking, Rests and
  //   BladeCfg gains the spin fields (spin, spin_grow, spin_max; `mode` absent = spinning); AbilityKind gains
  //   Clamp, Rewind, Lash, Spout, Offbeat, Snap, DoubleStop, Melody, Joke. Each stub `PORTED = false` (loads as
  //   Unsupported("<kind>")), as milestone 3's Task 3.
  // bf_sim::kinds::<kind>: State, is, setup, beat, tick, step, record_keys, snap (milestone 3's stub shape)
  // Ball gains, with Python's defaults: rad (Option<f64>: None = Rules.R), mass (Option<f64>), rw_left (f64),
  //   guard (f64), melody_until / melody_k0 / melody_rest, air already exists; Fight gains strings, surfs, spouts,
  //   x33_log, clamps, the per-kind state of each stub
  pub fn radius(f: &Fight, k: usize) -> f64;     // getattr(B, 'rad', R) at every site that reads it
  pub fn mass(f: &Fight, k: usize) -> f64;       // getattr(B, 'mass', 1.0)
  pub fn rewinding(f: &Fight, k: usize) -> bool; // getattr(B, 'rw_left', 0.0) > 0
  ```
- **What is wired**, each at the Python line it names and in Python's order, calling the stubs: `run()`'s setups for
  the nine kinds (5746-5911: `pulse`, `rewinders`, `grid16`, `inflate`, `tethers`, `wreckers`, `snappers`,
  `resters`); the beat triggers 5940-5945 (`trap`, `scratch_floor`, `tether`); the 16th counters (5946-5963 the
  `pulse` counter; `grid16_fire` 1487-1507; the `dc` counter's `surf` branch 5964-5974; `tether_16`, `wreck_tick`,
  `rests_tick`); `trigger`'s branches 2866-2869, 2874-2877, 2911, 2919-2929 and the spout branch of 2872; the rewind
  gate 3035; `move`'s per-ball radii 3179, masses 3209-3216, `clash_hooks` 3233 and the rewinding skips; `damage`'s
  `rw_left` 325-326 and `guard` 342-343; `hazards_step`'s strings 1417-1434; `surf_rake_step`'s surfs 588-609 and
  spouts 671-699; `blade_step`'s spin 780-825; `notes_step` 6192; `clamps_step`, `lash_step`, `rewind_step`,
  `snap_step`; `snap`'s keys (`RAD`, `WRECK` 5577, `SNAP` 5588, `REST` 5616, `X33_SPOUT` 6221). Every kit kind's
  contact code that Python writes as `getattr(O, 'rad', R)` or tests `rw_left` calls `radius` / `rewinding` there
  (the scan lists each site with its line; today they are all `R`, so no fight changes).
- **Pair rules.** None of the older kinds has a `Rules` flag the series sets per pair (checked: `Rules` takes only
  `dmg_side` and milestone 3's three). `pair_rules.rs` is unchanged; the test proves it stays empty for every pair
  that includes an older fighter.

- [ ] **Step 1: Write the failing tests.** `sockets_older.rs`: with `Fight::trace_sockets`, a fight of each older
  fighter against FLUTE logs its sockets in the order `sockets_order.txt` lists (transcribed from `fight.py` with
  each line); every milestone 1 and 3 fight is unchanged (`replay`, every `studio/replay/fighters/*.txt`, `prefix`,
  `coverage`); a roster entry of each of the nine weapon kinds and a spinning blade still loads as `Unsupported`;
  `radius` and `mass` give `Rules.R` and 1.0 for every kit ball; `pair_rules` is empty for the 10 x 30 pairs.
- [ ] **Step 2:** Run: FAIL. **Step 3:** Implement. **Step 4:** PASS, every replay file unchanged.
- [ ] **Step 5:** Commit `bf-sim: sockets for the older kinds; per-ball radius, mass and rewind`.

### Task 4: Shared helpers, picture opt-ins, per-fighter and per-theme stubs

**Files:**
- Create: `studio/crates/bf-content/src/fighters/<slug>/{mod,look,sound,voice,card,state}.rs` stubs for the ten and
  `studio/crates/bf-content/src/themes/<name>/{mod.rs,classic.rs}` stubs for the twelve (milestone 3 Task 5's stub
  shape: `None` / `false` / empty, `ported: false`); `bf-picture/src/title/patterns/{drums,synth,turntables}.rs`
  (stubs; the other seven older patterns exist); `studio/crates/bf-content/src/fighters/shared/older.rs` (episode
  helpers more than one older look calls: `annulus_polys`, `mask_of`, `blend`, `dart_quads`, `draw_dart`, the
  episodes' shard and solo-envelope helpers where two looks share them, each from the episode named in the file);
  `tools/studio_export/older_vectors.py`, `studio/reference/older/vectors/`, tests
  `bf-content/tests/older_helpers.rs`, `bf-picture/tests/older_optins.rs`
- Modify: `studio/crates/bf-content/src/{fighters/mod.rs,themes/mod.rs,cards.rs,picture.rs}` (the dispatch for 30
  fighters and 26 themes), `studio/crates/bf-picture/src/{look.rs,ring/*}`, `tools/studio_export/older/
  picture_patches.py`

**Interfaces:**
- **Picture opt-ins.** An older layer that a kit hook (`fighters/base.py`'s 40 hooks) can express goes in that hook. A
  layer no hook can express gets one named opt-in, added here once in both languages: in Python as a run-time patch of
  `kit/picture.py` registered in `picture_patches.py` (off unless a look asks for it), in Rust as a new `Look` method
  with a default that draws nothing; milestone 1 and 3 frames unchanged. Known candidates from Table O1, each
  confirmed or dropped by reading `kit/picture.py`: **ball scale** for an inflated bag (BAGPIPES; first try BASSOON's
  `sx` / `sy` state keys), **floor rotation** for the scratched record (TURNTABLES `floor_angle`), **rope wobble** for
  a swing (BARI SAX `rope_amp`; first try the `rope` hook), **over-ring rig** for the crane (BARI SAX; first try
  `over_ring`), **post effect** for the VHS rewind (TURNTABLES `post_fx`; first try `after_grade`). The task's report
  lists every opt-in made and every candidate that a kit hook covered.
- `fighters::<slug>` entry points as milestone 3's Template F; `MUSIC_FEEDS_FIGHT` false for all ten (none reads the
  music: checked against `kit/story.py`).

- [ ] **Step 1:** Export `older_vectors` (each shared helper on the arguments the ten episodes use, collected by
  wrapping them while each episode's own renderer draws one frame a second in its scratch copy; plus edge cases).
  **Step 2: Write the failing tests:** helpers identical; each opt-in off gives milestone 3's frames (`golden_frames`,
  every kit look's `look_frames`); each opt-in on, driven by a stub look, equals the Python patch's frames (hashes in
  `older/vectors/`). **Step 3:** FAIL. **Step 4:** Implement; PASS; `cargo check --workspace --all-targets`.
- [ ] **Step 5:** Commit `shared helpers and picture opt-ins for the older looks; stubs for 10 fighters, 12 themes`.

### Task 5: The older configs and the fight file

**Files:**
- Create: `tools/studio_export/older/configs/<name>/episode.py` for the ten configs of Table O3 and the four stand-ins
  of Q2 (`drift_phonk_a`, `drift_phonk_b`, `noir_jazz_a`, `noir_jazz_b`), `tools/studio_export/unit_checks/
  older_configs_checks.py`
- Modify (only if a config needs it): `studio/crates/bf-core/src/fightfile/{mod,read,write,tests}.rs`

**Interfaces:**
- Each config is a kit config the kit's `episode.load(<path>)` accepts as it is, from outside `projects/` (the loader
  takes a path). Fields: as Table O3 says; `SERIES` absent (no tag); `SHARD_SEED`, `FX_SEED` the episode's; the
  episode's `LINES`, `SOLO`, `ROLES`, `TITLE_HITS`, `TITLE_LINES` when it has them, read from its `timeline.py` and
  `audio.py` constants (named in a comment with the file and line).
- **The fight file stays format 2.** Every field these configs set must exist in format 2; if one does not, it is
  added as an optional field whose absence means today's behaviour (no format bump), with its own tests. Expected:
  none needed. `validate` refuses `FIGHT_BAR` other than 2, a per-fight roster pin and a missing VS card, each with a
  one-line message naming the field (Review Focus 6).
- The stand-in configs for `drift_phonk` and `noir_jazz` use two non-retired pairs each at the theme's tempo, key and
  loop: `drift_phonk_a` PIANO vs ELECTRIC, `drift_phonk_b` ELECTRIC vs TUBA; `noir_jazz_a` TRUMPET vs TENOR SAX,
  `noir_jazz_b` TRUMPET vs ALTO SAX (the episode's own lead and two saxes that are not retired), each at the first
  seed of a `search --stop 1` that fits.

- [ ] **Step 1: Write the failing tests.** `older_configs_checks.py`: each config loads, its `CHORDS` equal
  `chords_for` of its fields, its fight runs to a K.O., and the configs whose episode already used a 2-bar start and
  today's roster reproduce the episode's replay-guard line. `fightfile/tests.rs`: the three refusals above.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `older episodes as kit configs; the fight file refuses old timelines`.

### Task 6: `np.convolve`'s order (carried from milestone 3)

**Skip** when milestone 3's ledger shows the audit merged. Otherwise: `bf-sound`'s `dsp::convolve_same` follows the
order numpy 2.5.3's `np.convolve` computes on this PC (the ledger's BASS DRUM entry: 2,394 of 3,000 samples differ in
the last bit for a 48-tap kernel), measured, not tabled, with every milestone 1 and 3 soundtrack gate unchanged at the
16-bit master and the stems re-blessed only where the float bits move. Older voices and beds are suspected users (the
episodes' `A.conv` and filter helpers); the report lists every call site. **Size M.** Commit `bf-sound: convolve in
numpy's order`.

---

# Template F': port older fighter X

Milestone 3's Template F (its Files, Interfaces, references, Steps and gates) applies in full. The differences:

**Files (added to Template F's):** `tools/studio_export/older/fighters/<slug>.py` (the reference kit module),
`studio/reference/fighters/<slug>/origin.json`.

**Step 0: The reference kit module and Gate O** (reviewed by the controller before Step 1):
- [ ] Write `older/fighters/<slug>.py` with `PORTED = True`, a `Look(base.Look)` and a `Sound(base.Sound)`, moving the
  episode's layers into the kit's hooks: draw functions are imported from the episode's `styles.py` through the
  scratch copy (the episodes' own rule: "styles live there so crossovers can import them"), never retyped; the
  `render.py` layer code and the `audio.py` part are moved into the hooks with their names. Any layer no hook covers
  uses Task 4's opt-in (or stops with NEEDS_CONTEXT naming the layer). The model and sprites come from the
  fighter's `pp_bridge` module as milestone 3's do.
- [ ] Run `python -m tools.studio_export fighter_<slug> --origin` (Gate O); it writes `origin.json`. A layer or bus that
  differs is fixed in the module, never in a comparison. A difference the episode itself causes (an older ring patch
  the kit does not have, the old tail) is outside the window by construction; any other is reported with the frame,
  the layer and the pixel, and the controller decides.

**Then Steps 1 to 5 of Template F**, against the module, with these additions:
- *Fights:* opponent FLUTE (milestone 3's R16 / R34 reference handicap rule); scenarios as Template F, plus each
  link of Table O4 that names X (its extra scenarios are in the entry); the **copy sweep**: EUPHONIUM against X, three
  seeds each side, fight parity to the floor and every copied kind drawn by X's look through `setup_as` without a
  panic (M3-R3).
- *Picture:* the module's look in the kit's pipeline is the reference (`look_frames`, `title`, `sprites`); the hull
  log, glow types (`glow_types.py`) and `cfg_keys` as milestone 3.
- *Sound:* the module's part is the reference (`voice`, `sound`, `sound_stems`, `checks`); `bed_calls()` equals the
  hooks the module's part calls unguarded.
- *Storage:* recordings in the cache (the default after Task 1); the folder within 220 KB including `origin.json`.

**Commit subjects:** `reference: <NAME>'s kit module, proven against <episode>`, then Template F's four.

## Batch A: the kinds every other fighter must respect

### Task 7: BAGPIPES
**New:** weapon `inflate` (setup, start, step, clash; the `grid16_fire` branch); **per-ball radius and mass** through
Task 3's `radius` / `mass` at every site; record key `RAD`. Look: the swell (the ball drawn bigger; Task 4's ball-scale
opt-in or the `sx` / `sy` state), the tartan trail, the deflating flight; title `_tartan`. Part: the episode's own
chanter, drone and reeds synth (no `pp_bridge` voice call; the voice gate fingerprints the module's tone functions).
**Extra tests:** BAGPIPES against each of the 20 kit fighters, three seeds each side, prefix to the floor and every kit
kind's contact event seen against a swollen ball (Review Focus 1). **Size L.**

### Task 8: TURNTABLES
**New:** weapon `scratch_floor`; ability `rewind` (ready gate, zip back along its own path, intangible while
`rw_left`), every `rw_left` site through `rewinding`. Look: the record floor and its rotation (Task 4's opt-in), the
tonearm, BASS DROP, the VHS rewind; title `_grooves` (the episode's own pattern, ported into its pattern file). Part:
scratches, floor scratches, the rewind whirr, organ stabs. **Extra tests:** TURNTABLES against each of the 20 kit
fighters, three seeds each side (Review Focus 1); a rewind that would undo less than `min_undo` waits. **Size L.**

### Task 9: BARI SAX
**New:** weapon `wrecking` (aim, tick, swing, out over the ropes, `steer`; `_sails_over`); abilities `offbeat` (SKANK)
and `snap` (CABLE SNAP); records `WRECK`, `SNAP`. Look: from #39 v4 (crane, cable, hook, harness, swing trail, the
honk front, rope wobble, the snap target); title `_barisax`. Part: honks, glisses, the doppler swing, rope twang,
winch. **Extra tests:** scenarios against BASSOON (a hop that sails over, one caught at the hop's edge) and BASS
CLARINET (a swing aimed round the whirl), each link row at its time. **Size L.**

### Task 10: SYNTH
**New:** weapon `tether` (plugs, cables, volts, the leash); ability `lash`. Look: cables, jacks, the sawtooth arms;
title `_circuit` (the episode's own). Part: arpeggio notes per level, jack clicks, zips, the cable snap, the supersaw
sweep. **Extra tests:** scenarios against TUBA (the spit breaks the leash) and HORN (FINAL FLICK); a cable count at
`n_max`. **Size L.**

### Task 11: VIOLA
**New:** weapon `rests` (the counted rests, the rolling whole note, `_crush`, `notes_step`, the resting `guard`);
abilities `double_stop`, `melody`, `joke` (the stun on the enemy through milestone 3's `stunned`). Look: the rests and
their count, the notes and their shadows, the padded sprite (`render_padded`), the laugh; title `_viola`. Part: its
voice a fifth below the violin's, the rimshot, the crowd's laugh. **Extra tests:** scenarios against PIANO (a note
flattens keys) and against FLUTE with a rest taking `guard` x damage at its time. **Size L.**

### Task 12: Register batch A
As milestone 3's Task 13 for Tasks 7 to 11: `registry.rs` `PORTED`, the Match-up roster's chips, the workspace tests
that name an unported fighter (they pick one at run time), `studio/README.md`'s roster line. **Size S, controller.**

## Batch B

### Task 13: UKULELE
**New:** weapon `surf` (on the shared 16th counter; ACOUSTIC's `rake` branch is merged and must stay byte-identical);
ability `spout` (`x33.rs`'s branch, the logged path). Look: waves, their shadows, the waterspout; title `_surf`. Part:
strums in the stroke's direction, the wave crash, the water wind. **Extra tests:** ACOUSTIC's suite unchanged; a
scenario against ACOUSTIC with a surf and a rake on the same 16th. **Size L.**

### Task 14: VIOLIN
**New:** the spinning branch of `blade_step` (`spin_grow`, `spin_max`; the thrust branch unchanged). Look: from #42, as
#28 drew it (bow, its trail, PIZZICATO's darts); title `_violin`. Part: bowed notes on the bow's quarter turns,
pizzicato, the squeal. Voice: CELLO's `violin.bowed` and noise stream, already shared. **Extra tests:** TROMBONE's
suite unchanged (the blade file is shared); a spin reaching `spin_max`. **Size M.**

### Task 15: HARP
**New:** weapon `trap` (`lay_string`, the strings' half of `hazards_step`; ORGAN's `line` half unchanged). Look: the
strings, their twang, darts, ANGEL WINGS and the halo; title `_harp_strings`. Part: the arpeggios. **Extra tests:**
ORGAN's suite unchanged; a scenario against ORGAN with a string and a pipe row live at once; the owner crossing its own
string takes nothing. **Size M.**

### Task 16: ACCORDION
**New:** weapon `bite` (`grid16_fire` branch, `bite_fire`, `bite_snap`); ability `clamp`. Look: the jaws, the clamp
across the ring, SQUEEZEBOX's collar; title `_pleats`. Part: reed chords from its own synth. **Extra tests:** a clamp
against a shielded TUBA (blocked) and an unshielded FLUTE (lands). **Size M.**

### Task 17: DRUMS
**New:** weapon `pulse` (the 16th counter shared with `volley on16`, haste pulses at `sub_dmg`, `clears`). Look: the
drumhead ring, CRASH CYMBAL, DOUBLE KICK's beater; the VS panel plain unless the port finds a pattern. Part: the kit,
sticks, rolls. **Extra tests:** a pulse clearing PIANO's keys at its time. **Size M.**

### Task 18: Register batch B
As Task 12 for Tasks 13 to 17. **Size S, controller.**

---

# Template T': port older theme Y

Milestone 3's Template T applies in full. The differences:

**Files (added):** `tools/studio_export/older/moods/<name>.py` (the reference mood: a kit `Bed(M)` class with every
method in the episode's order, every random draw in the episode's order, `hooks` exactly the optional methods it
defines, its sections for the card, count-in, fight bars, tape stop, K.O., stinger, victory and the build into frame
0, mapped onto the kit's beat sheet), `studio/reference/themes/<name>/origin.json`.

**Step 0: the reference mood and Gate O** (reviewed before Step 1): the mood's buses with silent parts equal the
episode's bed stems on the episode's own run (Task 2's `mood_origin`); where the episode's sections sit on its own
timeline (FIGHT! on bar 1, its own tail), the mood keeps the episode's material and places it on the kit's beat sheet,
and Gate O compares it on the episode's timeline through the mood's `timeline` argument.

**Then Template T's steps**, against the mood, with: reference configs from Table O2 (Task 5's configs); the bed alone
under the R25 stand-in for any unported fighter; a **stand-in pair** variant (Review Focus 3): the bed alone on a
recording of two fighters not in its episode equals the mood's master; the card's line says what the bed leaves to the
fighters ("No drums of its own: the fighters keep the beat" for balkan fire). Budget 120 KB with `origin.json`.

**Commit subjects:** `reference: <name> mood, proven against <episode>`, then `bf-content: <name> theme, classic bed`.

- **19 `drift_phonk`** From #27. The origin run is #27's own fight (GUITAR vs PIANO v1) in Python only; the Rust gates
  use `drift_phonk_a` and `_b` (Q2). **Size S.**
- **20 `balkan_fire`** From #28. No drums of its own. Reference `drums_vs_violin`. **Size M.**
- **21 `noir_jazz`** From #29 (Python-only origin run with SAX). Plays the PIANO voice (`piano_chord`). Rust gates on
  `noir_jazz_a` and `_b`. **Size M.**
- **22 `sea_shanty`** From #30. Reference `bagpipes_vs_accordion`. **Size M.**
- **23 `dark_breakbeat`** From #31. Reference `turntables_vs_synth`. **Size M.**
- **24 `villain_march`** From #32 v3. Reference `tuba_vs_flute` (both kit fighters: the whole master can be compared
  with the real parts too). **Size M.**
- **25 `spaghetti_western`** From #33 v3. Reference `ukulele_vs_acoustic`. **Size M.**
- **26 `gothic_horror`** From #35 v4. Reference `organ_vs_harp`. **Size M.**
- **27 `brass_house`** From #36. No bass and no brass of its own. Reference `trombone_vs_tuba_brass_house`. **Size M.**
- **28 `ska_punk`** From #39 v4. 169.18 BPM, the fastest: if the template's second tempo (plus 14, snapped by
  `loop_bpm`) is outside the theme card's bar range, the variant uses minus 14 and the report says so. Reference
  `bassoon_vs_barisax`. **Size M.**
- **29 `industrial`** From #40 v2. `busy_windows` reads the recording at construction (as horror trap reads LIGHTS
  OUT). Reference `barisax_vs_bassclarinet`. **Size L.**
- **30 `orchestral_drill`** From #42. Plays the CELLO voice. Reference `viola_vs_violin`. **Size M.**

### Task 31: Register the themes
`PORTED_THEMES` += every theme task merged; `themes_for` covers the 26; the theme cards' tests; `studio/README.md`.
**Size S, controller.**

---

# Template P': pair gate for an older config

Milestone 3's Template P applies in full (files, references, the nine gates, M3-R47's `native_render`). The config is
Task 5's (`older/configs/<pair>/episode.py`), written as `studio/fights/<pair>.toml` field for field; its kit recording
is in the theme's cache (M3-R44). `pairs.json` holds the Table O4 rows the pair proves. The episode's own video is not
compared (Gate O already proved each fighter and the bed against it). **Commit subject:** `<a> vs <b>: the older
episode as a fight file, proven`. Each row of Table O3 (Tasks 32 to 41) is one task, **Size S**, except 33, 34, 39 and
40 (two links each), **Size M**.

---

# Integration

### Task 42: Starters for all 30, theme compatibility, crossovers

**Files:** `studio/crates/bf-content/src/starter.rs`, `themes/mod.rs` (`themes_for`), `tests/starters.rs`,
`tools/studio_export/crossovers.py`, `studio/reference/episodes/crossovers/`

**Interfaces:** milestone 3's `starter` and `themes_for`, unchanged in shape. A config pair starts from its config
(Table O3 adds ten); when a pair has two configs with the same left fighter (TROMBONE vs TUBA: `trombone_vs_tuba` and
`trombone_vs_tuba_brass_house`), the episode's own music wins (Q5); any other pair from milestone 3's home-config rule,
where an older fighter's home config is its Table O3 config.

- [ ] **Step 1:** Export six more crossovers, one per older theme family: BAGPIPES vs CELLO, TURNTABLES vs PIANO, BARI
  SAX vs TUBA, VIOLA vs HORN, UKULELE vs SYNTH, HARP vs EUPHONIUM (each the kit config this rule makes, the kit's first
  fit, its master SHA-256 and 8 frame hashes).
- [ ] **Step 2: Write the failing tests:** all 870 ordered pairs give a starter that `validate` accepts and that
  round-trips through TOML; the ten new config pairs equal their fight files in both orders; the six crossovers equal
  the kit; `themes_for` refuses exactly the combinations the kit would crash on (M3-R2, recomputed for the 26).
- [ ] **Step 3:** FAIL. **Step 4:** Implement; PASS. **Step 5:** Commit `starters for all 30 fighters; 26 themes`.

### Task 43: The app with all 30 fighters and 26 themes

As milestone 3's Task 60, for 30 and 26: the roster's sections hold the 30 cards with no chip saying "not available
yet"; the status bar reads "30 fighters, 26 themes"; the theme picker lists `themes_for` the pair; `retired.rs` runs
a whole flow for every config pair and the six crossovers and finds neither retired fighter in any fight file,
caption, folder name or log (Review Focus 4). Snapshots: Match-up with all 30 chips, Music for `viola_vs_violin`,
Render for `barisax_vs_bassclarinet`. Commit `bf-app: all 30 fighters and 26 themes`.

### Task 44: Full renders from the command line

As milestone 3's Task 61 for the ten Table O3 configs (the kit's own recording: every check, the mp4, cover and
caption) and the six crossovers (`search --stop 10`, then the first fit that passes every check, else M3-R47's parity
note). Table into `docs/studio/m4-measurements.md` §2. Commit `m4: full renders of every older pair`.

### Task 45: Measurements

`docs/studio/m4-measurements.md`: with the other lanes idle, `ballfight bench` on one fight of each config pair
(fights per second, frame time mean and 95th percentile, soundtrack build time; the heaviest looks named against
milestone 3's); every prefix window; the statistics tables; every Gate O exception; the reference set by folder and
the tripwire's margin; the models' total and the exe's size; the rulings taken. Commit `m4: measurements`.

### Task 46: Docs and the closing run

**Files:** `docs/studio/HANDOFF.md` (status: the Studio holds every fighter and theme of §10; Rust is the source of
truth), `studio/README.md`, `studio/docs/recording-format.md` (complete), `studio/docs/porting-an-older-fighter.md`
(what the ports taught), and, only with the owner's yes (Q4), one line in `projects/ballfight/README.md`.
- [ ] `python -m tools.studio_export cache all`, then `cargo test --workspace --release -- --include-ignored`; the
  Windows CI job green; `verify`; `budget`; `python tools/ballfight_replay_hash.py` and the kit-episode lines one by
  one, unchanged; `git status --short projects reelkit series` empty (or exactly the one approved line). Commit
  `milestone 4: docs and the closing run`.

---

## 8. Risks and open questions

Each item: what is at stake, what the controller decides, the recommended answer.

1. **Q-A A reference that is wrong in the same way as the port.** The reference module and the Rust port are written
   from the same episode, possibly by the same worker. *Recommended:* Gate O is reviewed by the controller before any
   Rust (the checkpoint in Section 7), and its comparisons are against the episode's own scripts, never the module.
2. **Q-B Layers the kit's hooks cannot express** (floor rotation, a bigger ball, a crane over the ring). *Recommended:*
   Task 4's one-opt-in-per-need rule, both languages at once, existing frames unchanged; a port that meets another
   stops with NEEDS_CONTEXT.
3. **Q-C Episode stems mixed with the bed.** Some episodes place a fighter's notes on a bed bus. *Recommended:* run the
   episode with that fighter's placements removed at run time for the bed's Gate O and name the patch in
   `origin.json`; never compare a mixed bus.
4. **Q-D Per-ball size and intangibility everywhere.** Missing one site gives a short prefix window in one pairing
   only. *Recommended:* the scenario sweeps of Tasks 7 and 8 against all 20 kit fighters; the pre-flight scan lists
   every `rad` and `rw_left` site.
5. **Q-E Old episodes' fights do not replay under the kit's 2-bar start.** *Recommended:* Table O3's rule (the kit's run
   of the config at the episode's seed is the reference recording); Gate O uses the episode's own run.
6. **Q-F Storage.** Section 4: about 143 to 145 MB at the end; the tripwire at 145.5 MB; reserve C1 if reached.
7. **Q-G Render speed** with the heaviest older looks (BARI SAX's crane, TURNTABLES' post effect). *Recommended:*
   measure in Task 45; no frame bit changes for speed; any speed work goes to milestone 6.
8. **Q-H Size.** Ten fighters (six L), twelve themes, ten pairs, plus the reference modules: about the size of
   milestone 3 again. *Recommended:* six lanes, batch A first; themes fill free lanes.

**Questions only the owner can answer** (with the default the plan uses until answered):

1. **How the older ten are proven.** The kit never drew or played them; their only reference is their old episode,
   drawn with an older ring. The plan moves each fighter's look and sound into today's series frame (today's ring,
   HUD, VS card and tail), proves those layers and its sound against the episode pixel for pixel and sample for
   sample, and then proves the Studio against that bit for bit. Whole old videos (for example #28 with its own ring
   and no VS card) are not re-created. *Default: yes, as above* (it is how every kit fighter was ported).
2. **The two themes from episodes with a retired fighter.** Drift phonk (#27: GUITAR vs PIANO v1) and noir jazz (#29:
   TRUMPET vs SAX) are on the spec's theme list. *Default: port both themes; their old fights are used only inside
   the Python proof; the Studio tests and offers them with other fighters* (PIANO vs ELECTRIC, TRUMPET vs TENOR SAX,
   and so on). Neither retired fighter is offered or ported.
3. **Which BARI SAX.** Today's roster BARI SAX (#39 v4: near a corner it turns its swing to where the full swing
   fits). #40's episode switched that off for its own fight. *Default: today's BARI SAX everywhere* (abilities never
   change per fight), so BARI SAX vs BASS CLARINET starts from #40's music and handicap but is a new fight.
4. **The kit README.** Once the port is complete, the spec (§10) says the kit README should say the Studio is now the
   source of truth; nothing under `projects/` has been edited so far. *Default: one line added to
   `projects/ballfight/README.md` in Task 46, only after a yes.*
5. **TROMBONE vs TUBA's starting music.** It now has two: the big top (the kit's example) and brass house (#36's own).
   *Default: brass house (the real episode's music); the big top stays a preset on the theme card.*

## 9. What milestone 4 proves, mapped to the spec

- **§10 "the ten older fighters, ported from their latest episode folders"** Where: Table O1, Template F' (Tasks 7 to
  11, 13 to 17), Gate O (Task 2).
- **§10 "the 12 older episodes' themes"** Where: Table O2, Template T' (Tasks 19 to 30).
- **§1 "All 30 fighters and all 26 themes are available. GUITAR and SAX never appear."** Where: Tasks 12, 18, 31, 42,
  43.
- **§1 "every ported fighter's frames match the Python kit's frames"** Where: Gate O (module equals episode),
  Template F' and P' (Studio equals module).
- **§6 prefix, coverage, statistics, self-replay; §7 identical frames; §8 voices, parts, classic arrangements; §9
  checks and encode** Where: Templates F', T', P'; Tasks 44, 46.
- **§12 reference tools, manifest, the 150 MB rule** Where: Tasks 1, 2, 46; Section 4.
- **§14 milestone 4 "the older ten fighters and 12 themes, from their episode folders"** Where: this plan.
- **§16 risk "the ten older fighters live in per-episode copies of the scripts"** Where: Task 2 (scratch copies,
  reference modules) and Gate O.
