# #39 v2 snapshot: Bassoon vs Bari Sax with BARI SAX v2 (WRECKING BALL)

Delivered Oct 26 (`../ballfight_bassoon_vs_barisax_v2.mp4`, seed 27: BASSOON wins at 50.99 s with 5.3 HP). On Oct 27
the bari got a detailed Pixel Pitch model and the roster's WRECKING BALL learned to swing out over the ropes (#40 v2),
so the episode was re-made as v3 in the parent folder. This folder keeps v2 re-renderable: the same sources with paths
one level deeper; `sim.py` patches v2's frozen BARI SAX entry (no swing over the ropes) into the roster at import;
`check/` keeps v2's sprite caches (git-ignored; without them the sprites are drawn from the new model).
`tools/ballfight_replay_hash.py` guards seed 27 here.

    python audio.py && python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py
