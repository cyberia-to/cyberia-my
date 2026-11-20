//! The offer of record — what the Holder was shown, and the one text their
//! neuron signs. It is kept apart from the sheets that print it
//! (`deal_docs.rs`) because it has a stricter duty: the same offer must read
//! the same on every machine, byte for byte, or the signature under it means
//! nothing. `tests/offer_record.rs` freezes it.

use crate::pay::{yearly_rent, Form, TERM_YEARS};

/// What the Holder chose, frozen into the offer.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub plot_id: String,
    pub plot_name: String,
    pub price_usd: f64,
    pub form: Form,
    pub premium_pct: u32,
    /// `person` | `company`
    pub party: String,
    /// `id` | `abroad`
    pub origin: String,
    /// dollars per CX on the day of the offer, when the fix was reachable
    pub cx_level: Option<f64>,
    pub neuron: String,
    pub ts_ms: f64,
}

impl Offer {
    pub fn share(&self) -> u32 {
        if self.form == Form::Upfront { 100 } else { self.premium_pct }
    }

    pub fn at_signing(&self) -> f64 {
        self.price_usd * self.share() as f64 / 100.0
    }

    pub fn rent_usd(&self) -> f64 {
        yearly_rent(self.price_usd, self.share())
    }

    pub fn rent_cx(&self) -> Option<f64> {
        self.cx_level.map(|l| self.rent_usd() / l)
    }

    /// The offer as plain lines, the same on every machine — what is hashed
    /// into a particle and signed. Whole dollars and whole CX: the figures a
    /// person reads are the figures under the signature.
    pub fn canonical(&self) -> String {
        let mut l = vec![
            "CYBER VALLEY · OFFER OF RECORD".to_string(),
            format!("flat: {}", self.plot_id),
            format!("page: https://cyberia.my/plot/{}/", self.plot_id),
            format!("leasehold: {TERM_YEARS} years"),
            format!("price_usd: {:.0}", self.price_usd),
            format!("form: {}", self.form.key()),
            format!("at_signing_pct: {}", self.share()),
            format!("at_signing_usd: {:.0}", self.at_signing()),
        ];
        if self.form == Form::Index {
            l.push(format!("rent_usd_per_year_today: {:.0}", self.rent_usd()));
            if let (Some(cx), Some(level)) = (self.rent_cx(), self.cx_level) {
                l.push(format!("rent_cx_per_year: {cx:.0}"));
                l.push(format!("cx_level_usd: {level:.4}"));
            }
        }
        l.push(format!("holder: {} · {}", self.party, self.origin));
        l.push(format!("neuron: {}", self.neuron));
        l.push(format!("offered: {}", stamp_utc(self.ts_ms)));
        l.join("\n")
    }
}

/// `2026-01-01 00:00 UTC` — the moment of the offer as every reader of the
/// text sees it. The browser's own timezone cannot be part of a signed text:
/// the same offer would then hash differently in Bali and in Berlin, so the
/// civil date is counted here from the epoch instead of asked of the host.
pub fn stamp_utc(ms: f64) -> String {
    let secs = (ms / 1000.0).floor() as i64;
    let (days, tod) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // days since the epoch → civil date (Howard Hinnant's algorithm, eras of
    // 400 years counted from 0000-03-01)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", tod / 3600, tod % 3600 / 60)
}
