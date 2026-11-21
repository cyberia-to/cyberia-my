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

#[test]
fn controls_are_forty_four_pixel_targets() {
    assert!(pass().contains("min-height: 44px"));
}

#[test]
fn the_header_strip_scrolls_instead_of_wrapping() {
    assert!(pass().contains(".cyberia-nav { flex-wrap: nowrap; overflow-x: auto;"));
}

#[test]
fn motion_yields_to_the_visitors_wish() {
    assert!(pass().contains("prefers-reduced-motion: reduce"));
}
