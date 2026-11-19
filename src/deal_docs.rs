//! The papers of one deal, made for one flat. The agreement and its annexes
//! are templates on the shelf (`/docs`); four of them are specific to the
//! flat or the deal, and this module fills them from what the page already
//! knows: Annex B from the flat's boundary, Annex C from the dossier's
//! inventory, Annex D from the offer the Holder was shown, Annex E from the
//! index the rent is counted in. Each is a printable sheet, and the offer
//! also has a canonical text — the thing the Holder's neuron signs.
//!
//! These are data sheets for reading and for the notary's draft. The
//! instrument in Bahasa Indonesia, drawn by the notary, is what binds.

use crate::deals::fmt_ts;
use crate::docs::{library, GATEWAY};
use crate::land::{area_m2, centroid, load_map, LandFlat};
use crate::pay::{yearly_rent, Form, TERM_YEARS};
use crate::plot_land::Frame;
use crate::terms::{bare_name, fmt_usd, land_use, terms};

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
        l.push(format!("offered: {}", fmt_ts(self.ts_ms)));
        l.join("\n")
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn sheet(title: &str, sub: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><title>{t}</title>
<style>
body{{font:14px/1.5 -apple-system,Segoe UI,Helvetica,Arial,sans-serif;color:#111;max-width:780px;margin:32px auto;padding:0 24px}}
h1{{font-size:22px;margin:0 0 2px}} h2{{font-size:13px;letter-spacing:.12em;text-transform:uppercase;color:#555;margin:26px 0 8px}}
.sub{{color:#555;margin-bottom:6px}} .draft{{border:1px solid #c98500;color:#8a5a00;padding:8px 12px;border-radius:6px;font-size:12px;margin:14px 0}}
table{{border-collapse:collapse;width:100%;font-size:13px}} td,th{{border-bottom:1px solid #ddd;padding:6px 8px 6px 0;text-align:left;vertical-align:top}}
th{{color:#555;font-weight:400}} td.n,th.n{{text-align:right;font-variant-numeric:tabular-nums}}
pre{{background:#f4f4f4;padding:12px;border-radius:6px;font-size:12px;white-space:pre-wrap;word-break:break-word}}
svg{{max-width:100%;height:auto;border:1px solid #ddd;border-radius:6px}} .foot{{margin-top:28px;color:#777;font-size:12px}}
@media print{{body{{margin:0}} .noprint{{display:none}}}}
</style></head><body>
<h1>{t}</h1><div class="sub">{s}</div>
<div class="draft">Data sheet generated for this flat and this deal, for reading and for the notary's draft. The instrument in Bahasa Indonesia, drawn by the notary, is the text that binds.</div>
{b}
<div class="foot">Cyber Valley · PT Cyber Valley Estate · cyberia.my · <span class="noprint"><a href="javascript:print()">print or save as PDF</a></span></div>
</body></html>"#,
        t = esc(title),
        s = esc(sub),
        b = body
    )
}

/// Flats whose boundary runs within a few metres of this one.
fn neighbours(flat: &LandFlat, frame: &Frame) -> Vec<String> {
    let mine: Vec<(f64, f64)> = flat.coords.iter().map(|c| frame.to_xy(c[0], c[1])).collect();
    let map = load_map();
    let mut out: Vec<String> = map
        .phase0
        .iter()
        .filter(|f| f.id != flat.id)
        .filter(|f| {
            f.coords.iter().any(|c| {
                let p = frame.to_xy(c[0], c[1]);
                mine.iter().any(|m| ((m.0 - p.0).powi(2) + (m.1 - p.1).powi(2)).sqrt() < 4.0)
            })
        })
        .map(|f| bare_name(&f.name))
        .collect();
    out.sort();
    out
}

/// Annex B — the boundary of this flat: corners, sides, neighbours, drawing.
pub fn annex_b(flat: &LandFlat) -> String {
    let frame = Frame::new(&flat.coords);
    let edges = frame.edges();
    let book = terms(&flat.id);
    let area = area_m2(&flat.coords);
    let mut rows = String::new();
    for (i, e) in edges.iter().enumerate() {
        let c = flat.coords[i];
        rows.push_str(&format!(
            "<tr><td>{}</td><td class=n>{:.7}</td><td class=n>{:.7}</td><td class=n>{:.1} m</td></tr>",
            i + 1,
            c[1],
            c[0],
            e.len_m
        ));
    }
    let nb = neighbours(flat, &frame);
    let (w, h) = (frame.svg_w(), frame.svg_h());
    let centre = centroid(&flat.coords).unwrap_or((0.0, 0.0));
    let surveyed = book
        .and_then(|t| t.measured_m2.map(|m| format!("{m:.2} m² surveyed on the ground, {}", t.measured_on)))
        .unwrap_or_else(|| "no ground survey yet — the figure below is the plan on the map".into());
    let body = format!(
        r##"<h2>The flat</h2>
<table><tr><th>flat</th><td>{name} ({id})</td></tr>
<tr><th>district</th><td>{zone}</td></tr>
<tr><th>land use</th><td>{lu}</td></tr>
<tr><th>area in plan</th><td>{area:.0} m² · perimeter {per:.0} m</td></tr>
<tr><th>survey</th><td>{surveyed}</td></tr>
<tr><th>centre</th><td>{lat:.6}, {lon:.6} (WGS 84)</td></tr></table>
<h2>Corners and sides</h2>
<table><tr><th>corner</th><th class=n>latitude</th><th class=n>longitude</th><th class=n>side to the next corner</th></tr>{rows}</table>
<h2>Neighbours</h2><p>{nb}</p>
<h2>Drawing · north up · 2 m grid</h2>
<svg viewBox="0 0 {w:.0} {h:.0}" xmlns="http://www.w3.org/2000/svg"><path d="{path}" fill="#e9f7e9" stroke="#0a7a0a" stroke-width="2" vector-effect="non-scaling-stroke"/></svg>"##,
        name = esc(&bare_name(&flat.name)),
        id = esc(&flat.id),
        zone = esc(&flat.zone),
        lu = land_use(&flat.id).label(),
        per = frame.perimeter_m(),
        lat = centre.1,
        lon = centre.0,
        nb = if nb.is_empty() { "no flat shares a boundary on the map".to_string() } else { esc(&nb.join(" · ")) },
        path = frame.outline_path(),
    );
    sheet(
        &format!("Annex B — Plot Boundary · {}", bare_name(&flat.name)),
        "Lampiran B — Batas Bidang · per Plot",
        &body,
    )
}

/// Annex C — the due diligence pack as handed over: every paper of the land
/// and of the Company by its content hash.
pub fn annex_c(flat: &LandFlat, read_ms: Option<f64>) -> String {
    let mut body = String::new();
    for key in ["land", "company"] {
        let Some(s) = library().iter().find(|s| s.key == key) else { continue };
        body.push_str(&format!("<h2>{}</h2><table><tr><th>paper</th><th>what it is</th><th>content hash (IPFS)</th></tr>", esc(&s.section)));
        for it in &s.items {
            let link = if it.cid.is_empty() {
                "arrives before signing".to_string()
            } else {
                format!("<a href=\"{GATEWAY}{c}\">{c}</a>", c = esc(&it.cid))
            };
            body.push_str(&format!("<tr><td>{}</td><td>{}</td><td style=\"word-break:break-all\">{}</td></tr>", esc(&it.title), esc(&it.what), link));
        }
        body.push_str("</table>");
    }
    body.push_str(&format!(
        "<h2>Record of handover</h2><p>{}</p>",
        match read_ms {
            Some(ts) => format!("The Holder marked the pack as read on {} in the browser that holds the deal.", fmt_ts(ts)),
            None => "The Holder has not marked the pack as read yet.".to_string(),
        }
    ));
    sheet(
        &format!("Annex C — Due Diligence Pack · {}", bare_name(&flat.name)),
        "Lampiran C — Paket Uji Tuntas · per deal",
        &body,
    )
}

/// Annex D — the offer of record: the page and the price shown to the Holder.
pub fn annex_d(offer: &Offer, signature: Option<&crate::signal::DocSignature>) -> String {
    let mut rows = format!(
        "<tr><th>flat</th><td>{} ({})</td></tr><tr><th>page shown</th><td>https://cyberia.my/plot/{}/</td></tr>\
         <tr><th>leasehold</th><td>{TERM_YEARS} years · the right to use the flat, never its title</td></tr>\
         <tr><th>price</th><td>{}</td></tr><tr><th>form</th><td>{}</td></tr><tr><th>at signing</th><td>{} ({}%)</td></tr>",
        esc(&offer.plot_name),
        esc(&offer.plot_id),
        esc(&offer.plot_id),
        fmt_usd(offer.price_usd),
        if offer.form == Form::Upfront { "the whole price at signing, no rent" } else { "a premium at signing, the rest as a yearly rent in CX" },
        fmt_usd(offer.at_signing()),
        offer.share(),
    );
    if offer.form == Form::Index {
        rows.push_str(&format!(
            "<tr><th>rent, each year</th><td>{}</td></tr>",
            match (offer.rent_cx(), offer.cx_level) {
                (Some(cx), Some(l)) => format!("{cx:.0} CX — {} at the fix of the offer's day, ${l:.4} per CX", fmt_usd(offer.rent_usd())),
                _ => format!("{} in today's dollars, written in CX at the fix of the signing day", fmt_usd(offer.rent_usd())),
            }
        ));
    }
    rows.push_str(&format!(
        "<tr><th>holder</th><td>{} · {}</td></tr><tr><th>offered</th><td>{}</td></tr>",
        if offer.party == "company" { "a company" } else { "a person" },
        if offer.origin == "id" { "Indonesia" } else { "from abroad" },
        fmt_ts(offer.ts_ms)
    ));
    let sig = match signature {
        Some(s) => format!(
            "<h2>Signature</h2><table><tr><th>particle of the text</th><td style=\"word-break:break-all\">{}</td></tr>\
             <tr><th>neuron</th><td style=\"word-break:break-all\">{}</td></tr><tr><th>public key</th><td style=\"word-break:break-all\">{}</td></tr>\
             <tr><th>signature (ADR-036)</th><td style=\"word-break:break-all\">{}</td></tr><tr><th>signed</th><td>{}</td></tr></table>",
            esc(&s.particle), esc(&s.neuron), esc(&s.pubkey_hex), esc(&s.sig_hex), fmt_ts(s.ts_ms)
        ),
        None => "<h2>Signature</h2><p>Not signed yet.</p>".to_string(),
    };
    let body = format!(
        "<h2>The offer</h2><table>{rows}</table><h2>The text under the signature</h2><pre>{}</pre>{sig}",
        esc(&offer.canonical())
    );
    sheet(
        &format!("Annex D — Offer of Record · {}", offer.plot_name),
        "Lampiran D — Penawaran Tercatat · per deal",
        &body,
    )
}

/// Annex E — the index the rent is counted in, with this deal's numbers.
pub fn annex_e(offer: &Offer) -> String {
    let rent = match (offer.rent_cx(), offer.cx_level) {
        (Some(cx), Some(l)) => format!(
            "<tr><th>rent</th><td>{cx:.0} CX a year for {TERM_YEARS} years</td></tr><tr><th>index on the offer's day</th><td>${l:.4} per CX</td></tr>\
             <tr><th>that rent today</th><td>{}</td></tr>",
            fmt_usd(offer.rent_usd())
        ),
        _ => format!("<tr><th>rent</th><td>{} a year in today's dollars, written in CX at the fix of the signing day</td></tr>", fmt_usd(offer.rent_usd())),
    };
    let body = format!(
        r#"<h2>This deal</h2><table>{rent}<tr><th>remainder carried as rent</th><td>{} of {}</td></tr></table>
<h2>The rule</h2>
<p>The rent is a number of CX, fixed for the term. On each anniversary the CX owed are converted into the payment currency at the published fix of that day. Nothing sits between the index and the invoice: no collar, no floor, no review.</p>
<pre>I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t) / P̄ᵢ(t−1)        I(2022-01-07) = $1
invoice(year) = rent in CX · I(anniversary)</pre>
<p>Eight legs at fixed weights, rebalanced every day at trailing 365-day average prices: BTC 20 · ETH 15 · CNY 15 · USD 15 · GOLD 15 · CU 10 · OIL 5 · UX 5. A leg whose average closes below a tenth of its peak leaves the basket; its weight goes pro rata to the rest.</p>
<p>The specification: <a href="https://github.com/cyberia-to/cx/blob/main/specs/century-index.md">cx/specs/century-index.md</a> · the daily fix: <a href="https://cx.cyberia.my">cx.cyberia.my</a></p>"#,
        fmt_usd(offer.price_usd - offer.at_signing()),
        fmt_usd(offer.price_usd),
    );
    sheet(
        &format!("Annex E — Century Index · {}", offer.plot_name),
        "Lampiran E — Century Index · per deal",
        &body,
    )
}

/// Open a generated sheet in a new tab, as a document of its own.
pub fn open_sheet(html: &str) {
    use wasm_bindgen::JsValue;
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(html));
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type("text/html;charset=utf-8");
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&parts, &bag) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(&url, "_blank");
    }
}
