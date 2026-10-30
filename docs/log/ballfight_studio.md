# Log: BALLFIGHT Studio (`docs/studio/PLAN.md`)

Append new entries at the end (newest last).

## Porting log (PLAN step 1: every fighter and mood into the kit)
Episode sessions port a fighter when a crossover first needs it; those below are done, so the porting session skips them
(the full list is in `projects/ballfight/README.md`, "Ported so far", and `kit/fighters/`).
- **TRUMPET (#29 v3): ported by #46 Trumpet vs Clarinet** (session_01CzTfqa, Oct 28) into `kit/fighters/trumpet.py`
  (#29's styles + render layers + its audio part, unchanged). Check config `examples/trumpet_vs_sax` (seed 191, FIGHT!
  on bar 1 via the new kit option `FIGHT_BAR`, #29's SERIES / SHARD_SEED / FX_SEED): replay hash identical to #29's own
  sim for seeds 191/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); 12 frames pixel-identical in a matched harness
  (#29 with the sax's drawn layers and the UI off; the kit with a stub sax carrying only #29's sax effects, #29's beat
  pulse and #29's real-time flare clock): combos, FANFARE, DOUBLE TONGUE, the HIGH C charge + blast, the shatter; the
  trumpet bus matches #29's to 1e-16. Not ported: #29's noir-jazz bed (a mood still to port) and the retired SAX.
- **EUPHONIUM (#38): ported by #48 Euphonium vs Trombone** (session_01CzTfqaiKkq, Oct 28) into `kit/fighters/euphonium.py`
  (#38's BELL MORTAR styles + render layers + its audio part, unchanged; the key-bound calls follow the config's KEY, C
  minor = #38's notes). Check config `examples/bassclarinet_vs_euphonium` (seed 1269, #38's DMG_SCALE / SERIES /
  SHARD_SEED / FX_SEED / TITLE_LINES, LINES = #38's MELODY + BASS_8THS): replay hash identical to #38's own sim for
  1269/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); 10 frames pixel-identical in a matched harness (#38 with the
  bass clarinet's drawn layers and the UI off; the kit with a stub bass clarinet carrying only #38's bass-clarinet effects,
  run in #38's particle / flare order): shells + landing circles, MARCHING ORDERS, both IDENTITY CRISIS bars (the amber
  bass clarinet in its ball), the NOT A TUBA rain, the shatter; the euphonium bus bit-identical to #38's (max diff 0).
  Kit additions (additive, defaults unchanged): the HUD holds for `barrage` / `copy`, a Look hook `ball_sprite` (the
  enemy's model in its ball) and `extra_sprites` (those views in the sprite cache). New with the port: what IDENTITY
  CRISIS copies is drawn by the enemy's own look module bound to the euphonium's side (so in amber), copied kinds only;
  #38's copies of the bass clarinet (slap, twin, shadow) still sound as #38 played them, their pictures wait for the
  BASS CLARINET port. Not ported: #38's dark-drumline bed (a mood still to port) and the BASS CLARINET.
- **BASSOON (#39 v4): ported by #52 Oboe vs Bassoon** (session_01CzTfqc, Oct 29) into `kit/fighters/bassoon.py`
  (#39's TIPTOE / BOCAL HOOK / MOUNTAIN KING styles + render layers + its audio part, unchanged; the key-bound calls
  follow the config's KEY, E minor = #39's notes; new: while OUT OF TUNE its notes play flat against a sour twin, the
  Grieg quote included, silent in #39 where nothing detunes). Check config `examples/bassoon_vs_barisax` (seed 77,
  #39's DMG_SCALE / DMG_SIDE / SERIES / SHARD_SEED / FX_SEED, LINES = #39's BASSOON_8THS): replay hash identical to
  #39's own sim for 77/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); matched harness (#39 with the bari's drawn
  layers, effects and the UI off; the kit with a stub bari, #39's beat pulse): 7 of 10 frames pixel-identical (corner
  bob, mid-hop stretch, backstab slashes, STACCATO, landing squash, MOUNTAIN KING's dance, the champion's solo hop
  region), the bassoon's own layers identical in all 10; the rest differ only in the bari's GROWL rope shake, its brass
  shatter and faint canvas light on old claw marks at 47.0 s (marks themselves identical). Kit additions (additive,
  defaults unchanged, replay guard identical): draw_ball squash / stretch via optional state keys `sx` / `sy`, an
  airborne depth bias `air_bias` and painter height `paint_h`, Look attribute `speed_lift`, HUD holds for `hook` /
  `mountain`. Not ported: #39 v2's leap over a swinging WRECKING BALL (waits for the BARI SAX port), #39's ska-punk bed.
