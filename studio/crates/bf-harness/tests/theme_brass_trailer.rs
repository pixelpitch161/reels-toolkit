//! The brass trailer against the Python kit: every gate of Template T (studio/docs/porting-a-fighter.md), with its
//! two voicing functions (the high tremolo's and the sequel's cello pad).

bf_harness::theme_suite!(
    "brass_trailer",
    [
        ("trem", bf_content::themes::brass_trailer::trem),
        ("pad_notes", bf_content::themes::brass_trailer::pad_notes),
    ]
);
