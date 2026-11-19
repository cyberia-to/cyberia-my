//! The plan's geometry: how long each side of a flat is, which way its sign
//! faces, and whether the frame contains the signs it sets. Pure arithmetic —
//! land.rs and plot_land.rs touch no browser, so they run natively here.

#[allow(dead_code)]
#[path = "../src/land.rs"]
mod land;
#[allow(dead_code)]
#[path = "../src/plot_land.rs"]
mod plot_land;

use plot_land::{Frame, Side};

const R_EARTH: f64 = 6_378_137.0;
/// plot_land's glyph advance, in fonts — kept in step by hand
const SIGN_CH: f64 = 0.45;

/// The lon/lat of a point so many metres east and north of (0, 0).
fn at(x_m: f64, y_m: f64) -> [f64; 2] {
    [(x_m / R_EARTH).to_degrees(), (y_m / R_EARTH).to_degrees()]
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

/// A 20 m square, turning both ways: four sides of 20 m, a perimeter of 80 m,
/// and every sign on the outside of the line it signs.
#[test]
fn a_known_square_has_four_signed_sides_facing_out_whichever_way_it_turns() {
    let mut ccw = vec![
        at(-10.0, -10.0),
        at(10.0, -10.0),
        at(10.0, 10.0),
        at(-10.0, 10.0),
    ];
    ccw.push(ccw[0]); // a closed ring repeats its first vertex
    let mut cw = ccw.clone();
    cw.reverse();
    for ring in [ccw, cw] {
        let frame = Frame::new(&ring);
        let sides = frame.sides();
        assert_eq!(
            sides.len(),
            4,
            "five coords, four sides — the repeat is dropped"
        );
        for s in &sides {
            assert!(close(s.len_m, 20.0), "a side of 20 m, got {}", s.len_m);
            assert_eq!(
                s.sign.as_deref(),
                Some("20.0 m"),
                "a 20 m side carries its sign"
            );
        }
        let perimeter = frame.perimeter_m();
        assert!(close(perimeter, 80.0), "perimeter of 80 m, got {perimeter}");
        // the sign leans away from the middle of the flat, never into it
        let centre = frame.to_svg(0.0, 0.0);
        for s in &sides {
            let away = (s.mid.0 - centre.0, s.mid.1 - centre.1);
            let dot = s.out.0 * away.0 + s.out.1 * away.1;
            assert!(
                dot > 0.0,
                "the normal {:?} of the side at {:?} points inward",
                s.out,
                s.mid
            );
        }
    }
}

/// The frame grows with the signs: on a flat wide enough that a sign outruns
/// the old fixed margin, every sign still stands inside the viewBox.
#[test]
fn the_frame_contains_the_signs_it_sets() {
    let mut ring = vec![
        at(-60.0, -60.0),
        at(60.0, -60.0),
        at(60.0, 60.0),
        at(-60.0, 60.0),
    ];
    ring.push(ring[0]);
    let frame = Frame::new(&ring);
    let (w, h) = (frame.svg_w(), frame.svg_h());
    let (font, off) = (frame.font(), frame.sign_off());
    assert!(
        font > 17.0,
        "a 120 m flat sets its signs larger than 17 units, got {font}"
    );
    for Side { mid, out, sign, .. } in frame.sides() {
        let text = sign.expect("a 120 m side carries its sign");
        let half = text.chars().count() as f64 * SIGN_CH * font / 2.0;
        let (x, y) = (mid.0 + out.0 * off, mid.1 + out.1 * off + font * 0.35);
        assert!(
            x - half >= -1e-6 && x + half <= w + 1e-6,
            "the sign {text} at {x} ± {half} leaves a frame {w} wide"
        );
        assert!(
            y - font >= -1e-6 && y + font <= h + 1e-6,
            "the sign {text} at {y} leaves a frame {h} tall"
        );
    }
}

/// A side shorter than its own sign is left bare, so the plan of a flat with
/// many short sides stays readable.
#[test]
fn a_side_too_short_for_its_sign_stays_bare() {
    // a 120 m square with a one metre notch cut into its north side
    let ring = vec![
        at(-60.0, -60.0),
        at(60.0, -60.0),
        at(60.0, 60.0),
        at(0.5, 60.0),
        at(0.5, 59.0),
        at(-0.5, 59.0),
        at(-0.5, 60.0),
        at(-60.0, 60.0),
    ];
    let sides = Frame::new(&ring).sides();
    let short: Vec<&Side> = sides.iter().filter(|s| s.len_m < 2.0).collect();
    assert_eq!(short.len(), 3, "the notch is three sides of one metre");
    for s in &short {
        assert!(
            s.sign.is_none(),
            "a side of {:.1} m is too short to sign, yet it carries {:?}",
            s.len_m,
            s.sign
        );
    }
    assert!(
        sides.iter().any(|s| s.len_m > 50.0 && s.sign.is_some()),
        "the long sides of the same flat are still signed"
    );
}
