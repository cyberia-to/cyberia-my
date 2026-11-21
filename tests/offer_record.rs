//! The offer of record is the one text a buyer's signature rests on: Annex D
//! prints it, the Company receives it, the notary draws the deed from it. It
//! is frozen here byte for byte, and the signed pair is checked both ways —
//! accepted over the text it covers, refused the moment one figure moves.

#![allow(dead_code, unused_imports)]

#[path = "../src/deals.rs"]
mod deals;
#[path = "../src/land.rs"]
mod land;
#[path = "../src/offer.rs"]
mod offer;
#[path = "../src/orders.rs"]
mod orders;
#[path = "../src/pay.rs"]
mod pay;
#[path = "../src/signal.rs"]
mod signal;
#[path = "../src/terms.rs"]
mod terms;
#[path = "../src/wallet.rs"]
mod wallet;

use offer::Offer;
use pay::Form;

/// the neuron of seed `[7; 32]` — the browser the envelope fixture was taken
/// from, in `tests/fixtures/domain-v1.json`
const NEURON: &str = "cyber1uj73l0ym4akga7mdc5qn7c4ytr8l2vlwxwkwed";

/// 2026-01-01 00:00:00 UTC
const OFFERED_MS: f64 = 1_767_225_600_000.0;

const FROZEN: &str = "\
CYBER VALLEY · OFFER OF RECORD
flat: sinwood-8
page: https://cyberia.my/plot/sinwood-8/
leasehold: 25 years
price_usd: 120000
form: index
at_signing_pct: 30
at_signing_usd: 36000
rent_usd_per_year_today: 11083
rent_cx_per_year: 7389
cx_level_usd: 1.5000
holder: person · abroad
neuron: cyber1uj73l0ym4akga7mdc5qn7c4ytr8l2vlwxwkwed
offered: 2026-01-01 00:00 UTC";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn an_offer() -> Offer {
    Offer {
        plot_id: "sinwood-8".into(),
        plot_name: "Sinwood 8".into(),
        price_usd: 120_000.0,
        form: Form::Index,
        premium_pct: 30,
        party: "person".into(),
        origin: "abroad".into(),
        cx_level: Some(1.5),
        neuron: NEURON.into(),
        ts_ms: OFFERED_MS,
    }
}

#[test]
fn the_offer_text_is_frozen_and_the_signature_covers_exactly_it() {
    let o = an_offer();
    assert_eq!(o.canonical(), FROZEN);
    // the stamp is counted from the epoch, never asked of the host's clock:
    // the same offer must hash the same in Bali and in Berlin
    assert_eq!(offer::stamp_utc(0.0), "1970-01-01 00:00 UTC");

    let key =
        mudra::domain::DomainKey::derive(&[7; 32], signal::DOMAIN, signal::HRP).unwrap();
    assert_eq!(key.bech32, NEURON);
    let text = o.canonical();
    let signed = signal::DocSignature {
        particle: signal::text_particle(&text),
        neuron: key.bech32.clone(),
        pubkey_hex: hex(&key.pubkey),
        sig_hex: hex(&mudra::claim::sign_arbitrary(
            key.signing_key(),
            &key.bech32,
            text.as_bytes(),
        )),
        ts_ms: OFFERED_MS,
    };
    assert!(signal::verify_document(&text, &signed));

    // one figure moves and the signature no longer stands under the offer
    let mut moved = an_offer();
    moved.premium_pct = 40;
    assert_ne!(moved.canonical(), text);
    assert!(!signal::verify_document(&moved.canonical(), &signed));

    // nor does it stand with the particle recomputed over the new text
    let forged = signal::DocSignature {
        particle: signal::text_particle(&moved.canonical()),
        ..signed
    };
    assert!(!signal::verify_document(&moved.canonical(), &forged));
}

/// `due_at_signing` is the one rule for the money a flat is due at signing:
/// the offer of record prints it as `at_signing_usd`, and the desk counts
/// "still due at signing" with it (`src/admin.rs`, `due_of`). The index form
/// is the branch no bench has walked — a premium of the price, not the whole
/// of it — so it is held here.
#[test]
fn the_money_due_at_signing_is_the_premium_under_the_index_form() {
    use pay::{due_at_signing, share_at_signing};
    // the index form: the premium at signing, and nothing of the remainder the rent carries
    assert_eq!(share_at_signing(Form::Index, 30), 30);
    assert_eq!(due_at_signing(26_500.0, Form::Index, 30), 7_950.0);
    assert_eq!(due_at_signing(120_000.0, Form::Index, 30), an_offer().at_signing());
    // upfront is the whole price, whatever premium rides along with it
    assert_eq!(share_at_signing(Form::Upfront, 30), 100);
    assert_eq!(due_at_signing(26_500.0, Form::Upfront, 30), 26_500.0);
    // a deal that names no premium is counted at the whole price; past 100 is still the whole price
    assert_eq!(due_at_signing(26_500.0, Form::Index, 100), 26_500.0);
    assert_eq!(due_at_signing(26_500.0, Form::Index, 140), 26_500.0);
    // and nothing is due on a price of nothing
    assert_eq!(due_at_signing(0.0, Form::Index, 30), 0.0);
    // the offer of record holds no second copy of the rule: it is the same call,
    // so it clamps where pay clamps — a premium past 100 is the whole price, once
    let mut hot = an_offer();
    hot.premium_pct = 140;
    assert_eq!(hot.share(), share_at_signing(Form::Index, 140));
    assert_eq!(hot.at_signing(), due_at_signing(120_000.0, Form::Index, 140));
    assert_eq!(hot.at_signing(), 120_000.0);
    let upfront = Offer { form: Form::Upfront, premium_pct: 30, ..an_offer() };
    assert_eq!(upfront.share(), share_at_signing(Form::Upfront, 30));
    assert_eq!(upfront.at_signing(), due_at_signing(120_000.0, Form::Upfront, 30));
}
