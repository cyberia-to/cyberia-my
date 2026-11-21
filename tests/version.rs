//! One version, in three places: the crate, the door's `version` meta, and the changelog's
//! top section. A bump that forgets one of them leaves the cockpit reading a door that claims a
//! version nobody released (`apparatus/scripts/cockpit.py` § served), so the gate is here.

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn the_door_and_the_changelog_carry_the_crates_version() {
    let html = include_str!("../index.html");
    let meta = format!("<meta name=\"version\" content=\"{VERSION}\" />");
    assert!(html.contains(&meta), "index.html does not state {VERSION}: expected the line `{meta}`");

    let log = include_str!("../CHANGELOG.md");
    let head = log
        .lines()
        .find(|l| l.starts_with("## "))
        .expect("CHANGELOG.md has no version section");
    assert!(
        head.starts_with(&format!("## {VERSION} ")),
        "CHANGELOG.md opens with `{head}`, not with {VERSION}"
    );
}

#[test]
fn the_version_is_three_numbers() {
    let parts: Vec<&str> = VERSION.split('.').collect();
    assert_eq!(parts.len(), 3, "a version has three numbers, never another shape: {VERSION}");
    assert!(parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())), "{VERSION}");
}
