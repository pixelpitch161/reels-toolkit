# BALLFIGHT Studio step 1: porting the fighters and moods into the kit (hand-off, Oct 27)

Step 1 of `docs/studio/PLAN.md`: every remaining BALLFIGHT fighter into `projects/ballfight/kit/fighters/<name>.py` and
every episode's mood into `projects/ballfight/kit/moods/<mood>.py`, each checked against its episode with an
`examples/<a>_vs_<b>/episode.py` that rebuilds it. The first session (session_01CzTfqdSsfsbYNYgocwEbMx) stopped after
TUBA at the user's request; this file is everything the next session needs. Its log: `docs/log/ballfight_studio.md`.

## Status

| Done | Checked against | How close |
|---|---|---|
| DRUMS, mood `balkan_fire` | #28 (`examples/drums_vs_violin`, FIGHT_BAR 1) | hash identical; the DRUMS' 'kit' + 'solo' buses and the bed's bass / accordion / pad / cimbalom buses sample-identical; frames: DRUMS layers match by eye (whole frames differ by design: #28 predates the card, v2 effects, HUD list, fx.Shake) |
| VIOLIN (as #42), VIOLA, mood `orchestral_drill` | #42 (`examples/viola_vs_violin`) | hash identical; 17 frames pixel-identical except the rope fix below; audio.wav identical 3.0-59.5 s (the last bar: 4 bowed notes' vibrato phase) |
| ORGAN (by the #45 session), PIANO v2 (#45) | #35 v4 / new | see `docs/log/ballfight_piano_vs_organ.md` |
| (in progress) TUBA, mood `brass_house` | #36 (`examples/trombone_vs_tuba`) | drafted (`kit/fighters/tuba.py`, `kit/moods/brass_house.py`, a `canvas_before` hook), being verified; if they are not on main yet, the last backup is on the branch `wip/kit-port` |

Not started (suggested order, most reused first; each line = one session-sized job):

1. Mood `villain_march` (#32 v3, `projects/ballfight_tuba_vs_flute`) + `examples/tuba_vs_flute` (TUBA + FLUTE are both
   ported: a full rebuild; #32 v3 has the card and v2 effects but no HUD ability list).
2. HORN + mood `brass_trailer`: #34 v5 (`projects/ballfight_trombone_vs_horn`, HORN v3 = ECHO CALL / FINAL FLICK, seed
   140, HUD list). TROMBONE is ported (as #41 drew it): a full rebuild.
3. HARP + mood `gothic_horror`: #35 v4 (`projects/ballfight_organ_vs_harp`, seed 237). ORGAN is ported: a full rebuild.
4. BASSOON + BARI SAX + mood `ska_punk`: #39 v4 (`projects/ballfight_bassoon_vs_barisax`, seed 77; the bari's latest
   is #39 v4: 10:34 Oct 27, after #40 v2 at 08:09; v1-v3 are snapshots in its v1/ v2/ v3/ subfolders: port v4).
5. BASS CLARINET + EUPHONIUM + mood `dark_drumline`: #38 (`projects/ballfight_bassclarinet_vs_euphonium`, seed 1269);
   the bass clarinet was last delivered in #40 v2: port #40's if it differs. Then mood `industrial` from #40 v2
   (`projects/ballfight_barisax_vs_bassclarinet`, seed 128; its sim.py pins the bari's `steer` off: put that in the
   example's RULES) + `examples/barisax_vs_bassclarinet`.
6. TRUMPET + mood `noir_jazz`: #29 v3 (`projects/ballfight_trumpet_vs_sax`, seed 191, FIGHT_BAR 1, no card). SAX is
   retired (never ported): the example replays with SAX as a bare, silent ball; check TRUMPET by the hash, its frames
   (by eye: #29 predates the card + HUD list) and its buses. Mood `drift_phonk` from #27 (`projects/ballfight`, GUITAR
   retired + PIANO v1 frozen in #27's sim.py: no kit rebuild is possible): check the bed bar by bar against #27's
   audio.py (same tempo, chords and bar intensities) and its buses where the fighters don't play.
7. BAGPIPES + ACCORDION + mood `sea_shanty`: #30 v3 (`projects/ballfight_bagpipes_vs_accordion`, seed 1442, card, no
   HUD list).
8. TURNTABLES + SYNTH + mood `dark_breakbeat`: #31 v3 (`projects/ballfight_turntables_vs_synth`, seed 1339, card, no
   HUD list).
9. UKULELE + mood `spaghetti_western`: #33 v3 (`projects/ballfight_ukulele_vs_acoustic`, seed 1130). ACOUSTIC is
   ported; update `examples/ukulele_vs_acoustic` (today it uses big_top as a stand-in bed) to the real mood.

Never port GUITAR or SAX (retired; their roster entries stay only so #27 and #29 replay).

When everything is in: the "Ported so far" line and the moods list in `projects/ballfight/README.md`, the docstrings of
`kit/fighters/__init__.py` and `kit/moods/__init__.py`, the log, and the final table for the user (each fighter and
mood: ported, verified against which episode, differences found).

## Rules (the user's, for this job)

- Port each fighter **as it was last delivered** (check its idea file's board history and `git log` for the latest);
  moods named by mood, taking tempo and chords as inputs so any pair can use any mood.
- Work one episode at a time: its fighters and mood, then `examples/<episode>/episode.py`. Verify with
  `python -m ballfight hash` against `tools/ballfight_replay_hash.py`, a few frames and audio.wav against the episode's
  own (PYTHONHASHSEED=0 on both). Add each example to KIT_EPISODES in the replay guard.
- **Small commits (one fighter or one mood per commit)**; `git pull --rebase origin main` before each push, push after
  each commit (other sessions push to the kit too). Re-check `kit/fighters/` before each port.
- Engine changes only if a port truly needs one; kit changes only additive (a new optional hook whose default keeps
  today's output), own commit, regression set identical first. Every finished episode keeps replaying identically.
- **Leave the episode folders as delivered** (the harness never writes into them except their git-ignored check/).
- If the session runs long, stop at a clean point and hand off.

## How a port goes

- **Look**: copy the fighter's draw functions from the episode's `styles.py` unchanged into its module; move each of
  its layers from the episode's `render.py` into the Look hook at the same point of the pipeline (they are listed in
  pipeline order in `kit/fighters/base.py`). Keep the source's arithmetic and order (float sums in another order
  change pixels). Effects (`P.FL / BU / PA / MK / MO`) are built once from `fx_event` in event order. Filter to the
  fighter's own events (check each event's tuple in `fight.py`'s `self.ev(...)`: 'block' is (t, 'block', victim,
  attacker, ...), kill_keys events are (t, why, key_owner, by, x, y, id)). Cache per-frame data on the frame context
  (`drums.rings`, `viola.now`); build anything that needs the camera or the captions lazily (`viola.NOTES`).
- **Sound**: its part of `audio.py` into the Sound hooks (title_blast, count_in, play, ko, fall_apart, stinger, solo,
  reassemble, last_bar); its bus = the episode's bus name; `bus_gain` / `send` / `band` / `colour_args` from the
  episode; per-bus placement order as the episode had it; pans mirrored by side; pitches from the timeline's harmony
  helpers, motifs moved with `key_shift` (unchanged in the source key); the config's TITLE_HITS / SOLO / LINES override.
- **Mood**: the bed into a `Bed` class with the sections `music.score()` calls (see `orchestral_drill.py`,
  `balkan_fire.py`); chord tables replaced by rules that give the same notes for the episode's chords. An episode from
  before the VS card gets new card sections (say so in the docstring).
- **Config**: as `examples/viola_vs_violin/episode.py` (kit-lineage) or `examples/drums_vs_violin/episode.py` (before
  the card: `FIGHT_BAR = 1`, `REASSEMBLE_AT`). LINES expanded over the LOOP and checked slot by slot against the
  episode's own line function.
- Kit hooks added on the way (all optional): Look `ball_tint`, `sprite_lit`, `laugh`, `flashes`, `spark_kinds`,
  `overlay`, rope() -> (t, k); Sound `buses`, `ko`, `own_block`, `play_first`; a fighter module's `DEFAULT_SOLO`; HUD
  hold kinds in `picture._hold`; timeline `FIGHT_BAR`, `REASSEMBLE_AT`, `T_PICKUP`, `CARD`; mood `doors_open_gain`,
  `count_whoosh_gain`; (#45) `light`, `fx_end`, `fringe_size`, `block_burst`, `block_spark`; (TUBA) `canvas_before`.

## Verify: the harness in `tools/ballfight_port/` (run from `projects/`, PYTHONHASHSEED=0)

    T=../tools/ballfight_port
    python -m ballfight hash <example>                            # == the replay guard's lines for the episode
    python $T/epshot.py ballfight_<a>_vs_<b> OUT/ep f1 f2 ...     # the episode's own renderer -> .npy + .png
    python $T/kitshot.py <example> OUT/kit f1 f2 ...              # the kit
    python $T/cmp.py frames OUT/ep OUT/kit                        # identical / n px differ, max, bbox
    python $T/epaudio.py ballfight_<a>_vs_<b> OUT/ep.wav OUT/ep.npz      # its audio, pre-master buses captured
    python $T/kitaudio.py <example> OUT/kit.wav OUT/kit.npz
    python $T/cmp.py wav OUT/ep.wav OUT/kit.wav                   # identical / max LSB diff
    python $T/cmp.py buses OUT/ep.npz OUT/kit.npz [t0 t1 [bus,..]] # per bus, max |diff|
    python $T/placelog.py ep|kit <folder|example> OUT.txt bus,..  # every Mix.place in order (diff the two)
    python $T/voicelog.py ep|kit <folder|example> OUT.txt         # Pixel Pitch notes synthesised, in order
    $T/regress.sh base BASE_DIR   (before a kit change)  /  $T/regress.sh check BASE_DIR OUT_DIR   (after)

Keep OUT dirs outside the repo (the session's scratchpad). A frame takes 20-40 s on 2 cores: render the two sides in
parallel, 10-16 frames at the key moments (card, count, first attacks, each ability's first use, the desperation move,
the K.O., the solo, the reassembly, 60.9 s). Find the cause of every difference (side-by-side + diff PNG) and fix the
port or explain it. Kit-lineage episodes (#34 v5 on: card + v2 effects + HUD list) should come out pixel- and
sample-identical; older ones (#28-#33) differ in the shared parts by design, so compare the fighter's layers by eye and
its buses numerically.

## Lessons

- **Pixel Pitch voices with a module-level RNG** (violin / viola / cello bowed vibrato, accordion60, drums60,
  organ60, piano): a note's samples depend on how many notes of that synth were made before it, so the order of
  first-time notes across the bed and both fighters must match the episode's (`Sound.play_first`); a residual that
  only an interleaved order would fix (e.g. #42's last bar) is acceptable: report where it is.
- **Source bugs**: #42 keyed VIOLA JOKE's rope wobble to the victim's index (the ropes shook at t = 1 s, under the
  doors): the port wobbles them when the note lands (with the bug the frames are pixel-identical, checked). #42's
  'crushed' pluck tested the wrong field and never sounded: left out.
- DRUMS was never redrawn after #28: no v2 effects (attack lights, bursts, particles) of its own yet.
- This container restarted every 20-35 min on Oct 27 (all running processes and background agents stop; files on disk
  survive): keep each command under ~10 min, write modules to disk early, commit + push each verified piece at once.
