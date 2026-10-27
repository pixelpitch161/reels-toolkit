"""The BALLFIGHT episode kit: one short config per episode (episode.py), everything else shared.

    episode.py   the config -> the episode's timeline (tempo, bars, title card, story targets, harmony helpers)
    story.py     the honest fight on the engine, cached; story analysis + the beat-sheet score
    tuning.py    handicap / roster-number grids          search.py   the early-stopping seed search
    picture.py   the renderer: ring, crowd, ball, HUD, captions, lens, post, title card + each fighter's look hooks
    music.py     the audio framework: voices, thinning, the mix, the master + the mood and each fighter's sound
    fighters/    one module per fighter: its look (draw style, effects, shatter parts) and sound (voice, idiom)
    moods/       reusable music beds (tempo and chords in)
    preview.py   a 6-8 s low-res clip of one fighter's signature and desperation move
    checks.py    focus, safe zone, levels, band, loudness, flash, loop        encode.py  the loop-clean mux
Run everything through `python -m ballfight <step> <episode>` (cli.py)."""
