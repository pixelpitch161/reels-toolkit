//! The user guide (`packaging/guide/index.html`): its pictures, its quoted labels, its length and its wording.

#[allow(dead_code, reason = "the guide uses only the drawing lock and the harness builders")]
#[path = "ui/common.rs"]
mod common;
#[allow(dead_code, reason = "the guide uses only the wording rule and the fixtures' texts")]
#[path = "ui/wording.rs"]
mod wording;

use std::collections::BTreeSet;
use std::path::PathBuf;

use common::gpu;

fn guide_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/guide")
}

fn page() -> String {
    std::fs::read_to_string(guide_dir().join("index.html")).expect("the guide's page")
}

/// Every piece of `html` between `open` and the next `close`.
fn between<'h>(html: &'h str, open: &str, close: &str) -> Vec<&'h str> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find(open) {
        let after = &rest[at + open.len()..];
        let Some(end) = after.find(close) else { break };
        found.push(&after[..end]);
        rest = &after[end + close.len()..];
    }
    found
}

/// `html` without its tags, its entities read.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The page's body, without its style.
fn body(html: &str) -> &str {
    between(html, "<body>", "</body>").first().copied().expect("a body")
}

#[test]
fn every_picture_exists() {
    let html = page();
    let used: BTreeSet<String> = between(&html, "<img src=\"", "\"")
        .into_iter()
        .map(str::to_string)
        .collect();
    assert!(!used.is_empty(), "the guide shows pictures");
    for src in &used {
        assert!(src.starts_with("pictures/"), "{src} is in pictures/");
        assert!(guide_dir().join(src).is_file(), "{src} is a file");
    }
    for entry in std::fs::read_dir(guide_dir().join("pictures")).expect("the pictures folder") {
        let name = format!("pictures/{}", entry.unwrap().file_name().to_string_lossy());
        assert!(used.contains(&name), "{name} is used by the guide");
    }
}

#[test]
fn every_quoted_label_is_in_the_app() {
    let _gpu = gpu();
    let html = page();
    let quoted: BTreeSet<String> = between(&html, "<b class=\"ui\">", "</b>")
        .into_iter()
        .map(|q| plain(q).trim().to_string())
        .collect();
    assert!(quoted.len() > 20, "the guide quotes the app ({})", quoted.len());
    let mut shown = BTreeSet::new();
    // The fixtures of every screen, and Music on Composed (the arrangement the guide's picture shows).
    let mut composed = bf_app::fixtures::music::vm();
    composed.draft.arrangement = Some(bf_app::state::COMPOSED.to_string());
    let screens = wording::fixtures().into_iter().map(|(_, vm)| vm).chain([composed]);
    for vm in screens {
        shown.extend(wording::texts(&wording::showing(vm)));
    }
    let missing: Vec<&String> = quoted
        .iter()
        .filter(|q| !shown.iter().any(|t| t.contains(q.as_str())))
        .collect();
    assert!(missing.is_empty(), "quoted but not in the app: {missing:?}");
}

#[test]
fn no_command_lines() {
    let html = page();
    for tag in ["<code", "<pre", "<kbd", "<script"] {
        assert!(!html.contains(tag), "no {tag}");
    }
    let text = plain(body(&html)).to_lowercase();
    let words: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '-'))
        .filter(|w| !w.is_empty())
        .collect();
    for bad in ["cargo", "git", "powershell", "cmd"] {
        assert!(!words.iter().any(|w| w.trim_end_matches('.') == bad), "no {bad:?}");
    }
    for (i, w) in words.iter().enumerate() {
        if w.ends_with(".exe") {
            let next = words.get(i + 1).copied().unwrap_or("");
            assert!(!next.starts_with('-') && !next.starts_with('/'), "{w} {next}");
        }
    }
    assert!(
        !text.contains(".exe -") && !text.contains(".exe /"),
        "no option after an .exe"
    );
}

#[test]
fn the_guide_is_short() {
    let html = page();
    let words = plain(body(&html)).split_whitespace().count();
    assert!(words <= 1500, "{words} words");
}

#[test]
fn the_wording_rule() {
    let html = page();
    let names = wording::name_words(&bf_app::fixtures::base::vm());
    let mut bad = Vec::new();
    for level in ["h1", "h2", "h3"] {
        for heading in between(&html, &format!("<{level}>"), &format!("</{level}>")) {
            let text = plain(heading);
            let broke = wording::breaks(text.trim(), &names);
            if !broke.is_empty() {
                bad.push(format!("{text:?}: {broke:?}"));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
