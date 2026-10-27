# #39 v1 snapshot: Bassoon vs Bari Sax with BARI SAX v1 (WINDOW RATTLER)

Delivered Oct 26 (`../ballfight_bassoon_vs_barisax_v1.mp4`, seed 190: BASSOON wins at 51.16 s with 4.8 HP). The user
found the bari boring, so the episode was redone with BARI SAX v2 (WRECKING BALL) in the parent folder. This folder
keeps v1 re-renderable: the same sources with paths one level deeper, and `sim.py` patches v1's frozen BARI SAX entry
(WINDOW RATTLER / GROWL / SKANK honks / LOW A) into the roster at import. The engine keeps the `rattle`, `offbeat` and
`quake` kinds, and `tools/ballfight_replay_hash.py` guards seed 190 here.

    python render.py && python audio.py && python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py
