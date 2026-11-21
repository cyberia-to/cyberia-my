//! The register of the land — the upper level the desk's FLATS menu groups
//! by. `src/cyberia_titles.json` is written by `scripts/sync-titles.py` from
//! the map layer's `certificates` folder and read by nobody after that, and
//! `titles::book()` folds a parse failure into an empty register: without
//! this test a broken file would show every flat under NO TITLE ON THE MAP
//! and say nothing. The figures are the layer as it was read on 2026-11-21.

#![allow(dead_code)]

#[path = "../src/titles.rs"]
mod titles;

#[test]
fn the_register_carries_every_parcel_and_the_flats_cut_from_it() {
    let book = titles::book();
    assert_eq!(book.titles.len(), 20, "20 parcels were read from the layer's certificates folder");

    let flats: Vec<&String> = book.titles.iter().flat_map(|t| t.plots.iter()).collect();
    assert_eq!(flats.len(), 108, "108 of the valley's 127 flats have their centre inside a mapped parcel");
    let mut once: Vec<&&String> = flats.iter().collect();
    once.sort();
    once.dedup();
    assert_eq!(once.len(), flats.len(), "a flat is cut from one parcel, never two");

    // every parcel says what it is; the register has nothing nameless in it
    for t in &book.titles {
        assert!(!t.name.is_empty(), "a parcel without a name: {t:?}");
    }

    // the largest, as the body of the pull request states it
    let sinwood = titles::title_of("sinwood-8").expect("sinwood-8 is cut from a mapped parcel");
    assert_eq!(sinwood.name, "sinwood");
    assert_eq!(sinwood.id, "4");
    assert_eq!(sinwood.doc, "certificate");
    assert_eq!(sinwood.plots.len(), 41);
    assert_eq!(sinwood.facts(), "parcel 4 · 400 ares declared · until 02.02.2053");
    assert_eq!(sinwood.held_line(), "HGB · PT Cyber Valley Estate");

    // and a flat outside every mapped parcel has no title, not a wrong one
    assert!(titles::title_of("avalon-15").is_none());
}
