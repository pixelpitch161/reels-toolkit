//! The big top against the Python kit: every gate of Template T (studio/docs/porting-a-fighter.md), with its two
//! voicing functions.

bf_harness::theme_suite!(
    "big_top",
    [
        ("voice", bf_content::themes::big_top::voice),
        ("organ", bf_content::themes::big_top::organ),
    ]
);
