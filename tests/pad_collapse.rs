//! The pad collapses to one chip and the choice outlives a reload. Three texts have to agree for that
//! to be true: the module's storage key, the task page that names the key, and the stylesheet that
//! collapses the pad under the class the module sets. A rename in one of them alone breaks the pad in
//! silence — it would open wide at every load, or collapse to nothing at all. The round trip through
//! `localStorage` itself is not exercised here: the crate carries no wasm test harness and the house
//! bans node, so the browser's side is first seen on the pull request's preview. The pattern is
//! `tests/version.rs`.

const PAD: &str = include_str!("../src/pad.rs");
const CSS: &str = include_str!("../style.css");
const TASK: &str = include_str!("../tasks/v2-pad-collapses.md");

/// the key the pad writes, and the one word it writes under it: "1" is hidden, and "1" is what the
/// reader accepts as hidden — a round trip is only a round trip while those two are the same word
#[test]
fn the_pad_writes_and_reads_the_same_key_the_task_names() {
    let key = PAD
        .lines()
        .find_map(|l| l.trim().strip_prefix("const PAD_HIDDEN_KEY: &str = \""))
        .and_then(|rest| rest.split('"').next())
        .expect("src/pad.rs no longer declares PAD_HIDDEN_KEY");
    assert!(
        TASK.contains(key),
        "tasks/v2-pad-collapses.md does not name `{key}`, the key the pad keeps its state under"
    );
    assert!(
        PAD.contains(r#"if hidden { "1" } else { "0" }"#),
        "the pad no longer writes \"1\" for hidden: the word written must be the word read"
    );
    assert!(
        PAD.contains(r#".map(|s| s == "1")"#),
        "the pad no longer reads \"1\" as hidden: the word read must be the word written"
    );
}

/// the class the module sets when the pad is put away, and the rule that puts it away
#[test]
fn the_class_the_pad_sets_when_hidden_is_the_class_the_style_collapses() {
    assert!(
        PAD.contains("class:np-hidden=move || pad_hidden.get()"),
        "src/pad.rs no longer marks the hidden pad with np-hidden"
    );
    assert!(
        PAD.contains(r#"class="np-toggle""#),
        "src/pad.rs no longer carries the np-toggle chip the collapsed pad is reduced to"
    );
    assert!(
        CSS.contains(".nav-pad.np-hidden > :not(.np-toggle) { display: none; }"),
        "style.css does not hide the keys of a collapsed pad, leaving only the np-toggle chip"
    );
}

/// pinned at the top-left corner of the map, on the desk and on a phone — the command's own word
#[test]
fn the_pad_is_pinned_to_the_top_left_corner() {
    let rule = CSS
        .lines()
        .find(|l| l.trim_start().starts_with(".nav-pad {"))
        .expect("style.css no longer carries the .nav-pad rule");
    assert!(
        rule.contains("left: 8px;") && rule.contains("top: 8px;"),
        "the pad is not pinned to the top-left corner: {rule}"
    );
    assert!(
        !rule.contains("translateY"),
        "the pad is still centred down the left edge instead of pinned to the corner: {rule}"
    );
}
