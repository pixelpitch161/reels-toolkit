# #39 v3 snapshot: Bassoon vs Bari Sax, the detailed bari swinging out over the ropes

Delivered Oct 27 (`../ballfight_bassoon_vs_barisax_v3.mp4`, seed 95: BASSOON wins at 50.80 s with 5.7 HP). The user
then pointed out the bari should get a full swing anywhere, not only mid-ring (near a corner the corner-post cap still
cut a quarter of its swings short), so the episode was re-made as v4 in the parent folder with the roster's new
'steer'. This folder keeps v3 re-renderable: the same sources with paths one level deeper; `sim.py` patches v3's
frozen BARI SAX (no 'steer') into the roster; `check/` keeps v3's sprite caches (git-ignored).
`tools/ballfight_replay_hash.py` guards seed 95 here.

    python audio.py && python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py
