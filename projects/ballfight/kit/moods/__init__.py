"""Reusable music beds (config MOOD = '<module>'): each a Bed class taking the episode's tempo and chords from the
timeline, with the sections kit.music calls in order (title, vs, title_rest, count_in, fight_start, fight, boom,
clash, ko, stinger, after_ko, victory, last_bars) and its own buses, reverb and compressor.

    big_top          #41 Flute vs Trombone: a sinister circus (calliope, bass drum, choked cymbal, snare, glock)
    baroque_storm    #37 Clarinet vs Oboe: harpsichord continuo, racing strings, timpani, thunder
    stadium_rock     #44 Electric vs Acoustic: minor arena rock in two arrangements that follow the leader (plugged:
                     fuzz rhythm guitar, pick bass, stadium kit; unplugged: strum shimmer, upright bass, hand percussion),
                     loudness-matched, crossfaded over a beat at each lead change; config PLUGGED names the plugged side
A new mood: copy one, keep its section methods, change the instruments and parts (or port an older episode's bed)."""
