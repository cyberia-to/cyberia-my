#[allow(dead_code)]
#[path="../src/signal.rs"]
mod signal;
#[test]
fn existing_browser_envelope_retains_identity_signature_and_storage_names(){
    let rows:serde_json::Value=serde_json::from_str(include_str!("fixtures/domain-v1.json")).unwrap();let v=&rows[0];
    assert_eq!(signal::DOMAIN,v["domain"]);assert_eq!(signal::HRP,v["hrp"]);
    let key=mudra::domain::DomainKey::derive(&[7;32],signal::DOMAIN,signal::HRP).unwrap();
    let hex=|bytes:&[u8]|bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
    assert_eq!(key.bech32,v["address"]);assert_eq!(hex(&key.pubkey),v["pubkey_hex"]);assert_eq!(hex(&key.native),v["native_hex"]);
    let body=v["body"].as_str().unwrap();
    let mut stored=signal::Signal{id:"sg-17".into(),neuron:key.bech32.clone(),state:"committed".into(),
        links:vec![signal::Link{from:"from".into(),rel:"rel".into(),to:"to".into(),weight:1.25,note:"annotation outside signed body".into()}],
        body_particle:hex(hemera::hash(body.as_bytes()).as_bytes()),pubkey_hex:v["pubkey_hex"].as_str().unwrap().into(),sig_hex:v["signature_hex"].as_str().unwrap().into(),note:"existing".into()};
    assert_eq!(signal::canonical_body(&stored.links),body);signal::verify_stored_signal(&stored).unwrap();
    let bytes=serde_json::to_vec(&stored).unwrap();let reopened=serde_json::from_slice(&bytes).unwrap();signal::verify_stored_signal(&reopened).unwrap();
    assert_eq!(stored.sig_hex,hex(&mudra::claim::sign_arbitrary(key.signing_key(),&key.bech32,body.as_bytes())));
    stored.links[0].weight=1.5;assert!(signal::verify_stored_signal(&stored).is_err());
    assert_eq!(signal::SEED_KEY,"cyberia_seed");assert_eq!(signal::SIGNALS_KEY,"cyberia_signals");assert_eq!(signal::SIGNAL_SEQ_KEY,"cyberia_signal_seq");
}
