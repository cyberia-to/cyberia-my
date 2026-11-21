//! The map's bytes, where this browser and the gate must agree. A changeset is a
//! file named by the hash of its canonical form, and `gate/layers.py` refuses a
//! change whose file is not exactly those bytes — so `atlas::canon` and `canon` of
//! the gate are one format written twice. The literal below is the one
//! `gate/layers_test.py` (`BytesTest`) asserts the gate writes; if the two ever
//! drift, one of these two tests fails instead of every change a person signs.
//!
//! Beside it: the caps measured before a signature is asked, what a KML and a
//! GeoJSON become on the way in, and the rules that paint a layer.

#![allow(dead_code, unused_imports)]

#[path = "../src/eth.rs"]
mod eth;
#[path = "../src/atlas.rs"]
mod atlas;

use atlas::{Draft, Feature};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// the same bytes as `BytesTest.GOLDEN` of `gate/layers_test.py`
const GOLDEN: &str = r#"{"layer":"0b7e3c1a-2f45-4c8d-9e6b-1a2b3c4d5e6f","put":[{"geom":[[1150890444,-82973027],[1150886763,-82977116],[1150887313,-82980103]],"id":"7a1d9e02-3b44-4f10-8c2e-5d6f70819a2b","kind":"building","prev":"sha256:abababababababababababababababababababababababababababababababab","props":{"base_cm":-150,"height_cm":420,"name":"Сосновый дом","note":"a wall \\ \"as drawn\""}}],"remove":["3c2f8d11-6a7b-4c5d-9e8f-0a1b2c3d4e5f"]}"#;
const GOLDEN_NAME: &str = "sha256:c066aacd18ed15687f771ab728ba64f0ce19c4b1bd83e8f023663364e08bb325";
const LAYER: &str = "0b7e3c1a-2f45-4c8d-9e6b-1a2b3c4d5e6f";

/// a building with a non-ascii name, a quote and a backslash in its note, a
/// negative integer and a version it replaces — `BytesTest.FEATURE` of the gate's suite
fn golden_feature() -> Feature {
    Feature {
        id: "7a1d9e02-3b44-4f10-8c2e-5d6f70819a2b".to_string(),
        kind: "building".to_string(),
        geom: vec![[1150890444, -82973027], [1150886763, -82977116], [1150887313, -82980103]],
        props: BTreeMap::from([
            ("name".to_string(), json!("Сосновый дом")),
            ("note".to_string(), json!("a wall \\ \"as drawn\"")),
            ("base_cm".to_string(), json!(-150)),
            ("height_cm".to_string(), json!(420)),
        ]),
        version: format!("sha256:{}", "ab".repeat(32)),
        ..Default::default()
    }
}

fn golden_draft() -> Draft {
    Draft {
        id: LAYER.to_string(),
        name: "the valley".to_string(),
        put: vec![golden_feature()],
        remove: vec!["3c2f8d11-6a7b-4c5d-9e8f-0a1b2c3d4e5f".to_string()],
        ..Default::default()
    }
}

#[test]
fn a_changeset_is_the_same_bytes_and_the_same_name_as_at_the_gate() {
    let file = atlas::changeset(&golden_draft());
    assert_eq!(file, GOLDEN);
    assert_eq!(atlas::name_of(&file), GOLDEN_NAME);
    // nothing of the layer's own draft leaks into the file, and one space breaks the name
    assert_ne!(atlas::name_of(&format!("{file} ")), GOLDEN_NAME);
}

#[test]
fn canon_sorts_the_keys_writes_no_space_and_leaves_non_ascii_alone() {
    let v = json!({ "b": 1, "a": [{ "d": "x", "c": -2 }], "z": [], "ä": "ö" });
    assert_eq!(atlas::canon(&v), r#"{"a":[{"c":-2,"d":"x"}],"b":1,"z":[],"ä":"ö"}"#);
}

#[test]
fn a_change_is_measured_in_bytes_against_the_gates_caps() {
    assert!(atlas::fits(&atlas::changeset(&golden_draft())));

    // a single feature can pass the file cap on its own: the count of features says nothing
    let mut huge = golden_feature();
    huge.props.insert("note".to_string(), json!("x".repeat(atlas::MAX_FILE)));
    let file = atlas::changeset(&Draft { put: vec![huge], ..Default::default() });
    assert!(file.len() > atlas::MAX_FILE);
    assert!(!atlas::fits(&file));

    // and the body cap bites first: there the file is a json string, where every
    // quote of it costs two bytes
    let mut quoted = golden_feature();
    quoted.props.insert("note".to_string(), json!("\"".repeat(1_000_000)));
    let file = atlas::changeset(&Draft { put: vec![quoted], ..Default::default() });
    assert!(file.len() < atlas::MAX_FILE);
    assert!(!atlas::fits(&file));
}

const KML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2"><Document><name>valley</name>
<Folder><name>fences</name>
<Placemark><name>north fence</name><description><![CDATA[east &amp; west]]></description>
<LineString><coordinates>115.1,-8.3,0 115.2,-8.4,0</coordinates></LineString></Placemark>
</Folder>
<Folder><name>Сосновый лес</name>
<Placemark><name>the pine plot</name>
<Polygon><outerBoundaryIs><LinearRing><coordinates>
115.0,-8.0,0 115.1,-8.0,0 115.1,-8.1,0 115.0,-8.0,0
</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>
</Folder>
<Placemark><name>the spring</name><Point><coordinates>115.05,-8.05,0</coordinates></Point></Placemark>
</Document></kml>
"#;

#[test]
fn a_kmls_folders_become_layers_and_what_lies_loose_becomes_one() {
    let got = atlas::from_kml(KML, "valley");
    assert_eq!(
        got.iter().map(|(n, f)| (n.as_str(), f.len())).collect::<Vec<_>>(),
        vec![("fences", 1), ("Сосновый лес", 1), ("valley", 1)]
    );

    let fence = &got[0].1[0];
    assert_eq!(fence.kind, "line");
    assert_eq!(fence.geom, vec![[1151000000, -83000000], [1152000000, -84000000]]);
    assert_eq!(fence.text("name"), "north fence");
    assert_eq!(fence.text("note"), "east & west"); // the cdata is unwrapped, the entity read

    let plot = &got[1].1[0];
    assert_eq!(plot.kind, "plot");
    // the ring's closing corner is dropped: the same ring is the same bytes
    assert_eq!(plot.geom, vec![[1150000000, -80000000], [1151000000, -80000000], [1151000000, -81000000]]);

    let spring = &got[2].1[0];
    assert_eq!(spring.kind, "point");
    assert_eq!(spring.geom, vec![[1150500000, -80500000]]);
}

const GEOJSON: &str = r#"{"type":"FeatureCollection","features":[
 {"type":"Feature","properties":{"name":"Сосновый дом","height_cm":450,"Status ":"sold","_skip":1},
  "geometry":{"type":"Polygon","coordinates":[[[115.0,-8.0],[115.1,-8.0],[115.1,-8.1],[115.0,-8.0]]]}},
 {"type":"Feature","properties":{"title":"the spring","description":"cold"},
  "geometry":{"type":"Point","coordinates":[115.05,-8.05]}}
]}"#;

#[test]
fn a_geojson_becomes_features_with_whole_coordinates() {
    let got = atlas::from_geojson(GEOJSON);
    assert_eq!(got.len(), 2);

    let house = &got[0];
    // a polygon carrying a height is a building
    assert_eq!(house.kind, "building");
    assert_eq!(house.text("name"), "Сосновый дом");
    assert_eq!(house.num("height_cm"), Some(450));
    assert_eq!(house.text("status_"), "sold"); // the key is folded, its space with it
    assert!(!house.props.contains_key("_skip")); // a key that starts with no letter is dropped
    assert_eq!(house.geom, vec![[1150000000, -80000000], [1151000000, -80000000], [1151000000, -81000000]]);

    let spring = &got[1];
    assert_eq!(spring.kind, "point");
    assert_eq!((spring.text("name"), spring.text("note")), ("the spring".to_string(), "cold".to_string()));
    assert_eq!(spring.geom, vec![[1150500000, -80500000]]);
}

#[test]
fn a_rule_reads_the_props_and_what_the_map_knows_of_a_feature() {
    let mut f = Feature {
        kind: "plot".to_string(),
        geom: vec![[1150000000, -80000000], [1151000000, -80000000], [1151000000, -81000000]],
        props: BTreeMap::from([("status".to_string(), json!("sold"))]),
        ..Default::default()
    };
    f.standing.confirm = 2;

    assert!(atlas::holds(&f, "status == sold"));
    assert!(!atlas::holds(&f, "status == open"));
    assert!(atlas::holds(&f, "status ~ SOL and confirm > 0"));
    assert!(atlas::holds(&f, "status")); // a key alone holds when it is there and not 0
    assert!(!atlas::holds(&f, "dispute")); // nobody disputed it
    assert!(!atlas::holds(&f, "no_such_prop == x"));

    // the measures the map takes of the shape itself, not props anyone wrote
    assert!(atlas::holds(&f, "kind == plot and area_m2 > 1000"));
    assert!(!atlas::holds(&f, "area_m2 > 100000000"));
}
