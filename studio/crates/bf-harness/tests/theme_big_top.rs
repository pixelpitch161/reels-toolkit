//! The big top against the Python kit: every gate of Template T (studio/docs/porting-a-fighter.md), and its two
//! voicing functions.

bf_harness::theme_suite!("big_top");

#[test]
fn voicings() {
    use bf_content::themes::big_top;
    bf_harness::theme::voicings(NAME, &[("voice", big_top::voice), ("organ", big_top::organ)]);
}
