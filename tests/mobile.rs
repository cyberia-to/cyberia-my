//! The phone's pass is in the stylesheet and the door lets it work: the viewport covers the notch,
//! inputs are 16 px (a phone does not zoom on focus), controls are 44 px targets, the header strip
//! does not wrap, and motion yields to `prefers-reduced-motion`. Text checks — no browser needed.

const CSS: &str = include_str!("../style.css");
const HTML: &str = include_str!("../index.html");

/// The stylesheet from the mobile pass to its end.
fn pass() -> &'static str {
    let at = CSS.find("mobile first, one pass").expect("style.css lost the mobile-first pass");
    &CSS[at..]
}

#[test]
fn the_viewport_covers_the_screen_and_the_notch() {
    assert!(HTML.contains("width=device-width"));
    assert!(HTML.contains("viewport-fit=cover"));
}

#[test]
fn the_braces_balance() {
    let open = CSS.matches('{').count();
    let close = CSS.matches('}').count();
    assert_eq!(open, close, "style.css has {open} '{{' and {close} '}}'");
}

#[test]
fn inputs_are_sixteen_pixels_on_a_phone() {
    assert!(pass().contains("input, select, textarea { font-size: 16px;"));
}

/// Every `.rs` view under `src/`, read as one string: the markup the stylesheet must match.
fn views() -> String {
    let mut all = String::new();
    for entry in std::fs::read_dir("src").expect("the tests run from the crate root") {
        let path = entry.expect("src/ is readable").path();
        if path.to_string_lossy().ends_with(".rs") {
            all.push_str(&std::fs::read_to_string(&path).expect("a view is readable"));
        }
    }
    all
}

/// The one line of the pass that carries the rule naming `anchor`.
fn rule(anchor: &str) -> &'static str {
    pass()
        .lines()
        .find(|line| line.contains(anchor))
        .unwrap_or_else(|| panic!("the pass has no rule naming {anchor}"))
}

#[test]
fn the_forty_four_pixel_rule_names_controls_the_views_render() {
    let line = rule(".cta-btn, .intent-commit");
    assert!(line.contains("min-height: 44px"), "the target rule lost its 44 px: {line}");
    let views = views();
    for class in [
        "cta-btn",
        "intent-commit",
        "globe-city",
        "dl-step",
        "pd-cta",
        "dl-go",
        "dl-mini",
        "ad-tile",
        "mp-links",
    ] {
        assert!(line.contains(&format!(".{class}")), "the 44 px rule does not name .{class}");
        assert!(views.contains(class), "no view under src/ renders .{class}");
    }
}

#[test]
fn the_pass_styles_nothing_the_views_stopped_rendering() {
    let views = views();
    for dead in ["cab-btn", "cab-link", "cab-flat", "cab-title", "cab-steps", "cf-facts", "gr-x"] {
        if !views.contains(dead) {
            assert!(!pass().contains(dead), "the pass styles .{dead}, which no view renders");
        }
    }
}

#[test]
fn the_close_button_is_a_forty_four_pixel_square() {
    let line = rule(".panel-x");
    assert!(line.contains("width: 44px"), "the close button is not 44 px wide: {line}");
    assert!(line.contains("height: 44px"), "the close button is not 44 px tall: {line}");
}

#[test]
fn the_header_strip_scrolls_instead_of_wrapping() {
    assert!(pass().contains(".cyberia-nav { flex-wrap: nowrap; overflow-x: auto;"));
}

#[test]
fn motion_yields_to_the_visitors_wish() {
    assert!(pass().contains("prefers-reduced-motion: reduce"));
}
