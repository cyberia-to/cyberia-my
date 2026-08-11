//! Earth states list + Bank Above Banks mock dashboard.
//! Protocol: ~/cyber/cyberia/protocol/bank-above-banks.md

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use leptos::prelude::*;
use serde::Deserialize;

const EARTH_JSON: &str = include_str!("earth_states.json");

/// Mock ETH/USD for display only — not a live oracle.
const MOCK_ETH_USD: f64 = 3_200.0;
/// Constitutional haircut (reserve volatility over redemption horizon).
const MOCK_H: f64 = 0.22;
/// SCR buffer β.
const MOCK_BETA: f64 = 0.30;
/// First-loss equity tranche (ETH).
const MOCK_EQUITY_ETH: f64 = 12_000.0;

#[derive(Clone, Debug, Deserialize)]
struct EarthFile {
    count: u32,
    states: Vec<EarthState>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct EarthState {
    name: String,
    code: String,
    slug: String,
    flag: String,
    region: String,
    population: u64,
    land_area_km2: u64,
    currency_code: String,
    currency_name: String,
    money_supply_b_usd: f64,
    #[serde(default)]
    money_supply_b_usd_prev: f64,
    #[serde(default)]
    token_price_usd: f64,
}

fn load_earth() -> EarthFile {
    serde_json::from_str(EARTH_JSON).expect("earth_states.json")
}

/// Format capital from billions USD → $1.2T / $340B / $1.6M …
fn fmt_cap(b_usd: f64) -> String {
    if b_usd <= 0.0 {
        return "soon".into();
    }
    let usd = b_usd * 1e9;
    let (v, s) = if usd >= 1e12 {
        (usd / 1e12, "T")
    } else if usd >= 1e9 {
        (usd / 1e9, "B")
    } else if usd >= 1e6 {
        (usd / 1e6, "M")
    } else if usd >= 1e3 {
        (usd / 1e3, "k")
    } else {
        (usd, "")
    };
    let num = if v >= 100.0 {
        format!("{v:.0}")
    } else if v >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    };
    format!("${num}{s}")
}

fn fmt_pop(n: u64) -> String {
    let f = n as f64;
    if f >= 1e9 {
        format!("{:.2}B", f / 1e9)
    } else if f >= 1e6 {
        format!("{:.1}M", f / 1e6)
    } else if f >= 1e3 {
        format!("{:.0}k", f / 1e3)
    } else {
        format!("{n}")
    }
}

fn fmt_eth(v: f64) -> String {
    if v.abs() >= 1000.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

fn cap_delta_pct(cur: f64, prev: f64) -> Option<f64> {
    if prev <= 0.0 || cur <= 0.0 {
        return None;
    }
    let d = (cur - prev) / prev;
    if d.abs() < 0.00005 {
        None
    } else {
        Some(d)
    }
}

fn fmt_delta(d: f64) -> (String, &'static str) {
    let pct = d * 100.0;
    let s = if pct.abs() >= 10.0 {
        format!("{pct:+.1}%")
    } else if pct.abs() >= 1.0 {
        format!("{pct:+.2}%")
    } else {
        format!("{pct:+.3}%")
    };
    if d > 0.0 {
        (s, "var(--cyber-green)")
    } else {
        (s, "var(--cyber-red)")
    }
}

fn hash01(s: &str) -> f64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    ((h >> 11) as f64) / ((1u64 << 53) as f64)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortKey {
    CapDesc,
    CapAsc,
    Name,
    Region,
    PopDesc,
    FloatDesc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Board {
    Earth,
    Bank,
}

/// Synthetic FX spoke — mock escaped float against ETH reserve.
#[derive(Clone, Debug)]
struct SpokeMock {
    rank: usize,
    flag: String,
    name: String,
    code: String,
    slug: String,
    region: String,
    currency: String,
    ctoken: String,
    /// Escaped float in ETH-value terms (F_i · P_i).
    float_eth: f64,
    /// Drainage quota α_i share of reserve capacity.
    alpha: f64,
    /// taker | maker | transition
    regime: &'static str,
    window: bool,
    hub: bool,
    /// Basis to external (bp) — honesty metric 3 seed.
    basis_bp: f64,
    /// Cap rank weight from cyberstates.
    cap_b_usd: f64,
}

/// Bank-level mock from protocol §2 / §7 / §8.
#[derive(Clone, Debug)]
struct BankMock {
    equity_eth: f64,
    reserve_eth: f64,
    haircut: f64,
    beta: f64,
    f_max_eth: f64,
    float_eth: f64,
    scr: f64,
    pi_true_eth: f64,
    fcf_eth: f64,
    basis_bp: f64,
    windows: usize,
    hubs: usize,
    spokes: Vec<SpokeMock>,
}

fn build_bank_mock(states: &[EarthState]) -> BankMock {
    // Earth countries with capital, ranked by cap.
    let mut ranked: Vec<&EarthState> = states
        .iter()
        .filter(|s| s.money_supply_b_usd > 0.0)
        .collect();
    ranked.sort_by(|a, b| {
        b.money_supply_b_usd
            .partial_cmp(&a.money_supply_b_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let equity = MOCK_EQUITY_ETH;
    let f_max = equity / (MOCK_H + MOCK_BETA);
    // Deploy ~62% of float capacity in mock Phase-0.
    let target_float = f_max * 0.62;

    // Weight float by log capital so long tail gets spokes without blowing SCR.
    let weights: Vec<f64> = ranked
        .iter()
        .map(|s| (s.money_supply_b_usd.max(0.01).ln()).max(0.1))
        .collect();
    let wsum: f64 = weights.iter().sum::<f64>().max(1e-9);

    let hub_set = [
        "USD", "EUR", "CNY", "JPY", "GBP", "BRL", "MXN", "ZAR", "SGD", "AED", "RUB", "INR", "KRW",
        "AUD", "CHF", "CAD", "TRY", "IDR", "THB", "NGN",
    ];

    let mut spokes: Vec<SpokeMock> = ranked
        .iter()
        .zip(weights.iter())
        .enumerate()
        .map(|(i, (s, w))| {
            let float_eth = target_float * (*w / wsum);
            let alpha = float_eth / f_max.max(1e-9);
            let h = hash01(&s.code);
            let regime = if i < 12 {
                "taker"
            } else if i < 40 {
                if h > 0.72 {
                    "transition"
                } else {
                    "taker"
                }
            } else if h > 0.55 {
                "maker"
            } else {
                "taker"
            };
            let window = i < 8 || (hub_set.contains(&s.currency_code.as_str()) && i < 25);
            let hub = hub_set.contains(&s.currency_code.as_str()) && i < 30;
            let basis_bp = match regime {
                "maker" => (h - 0.5) * 80.0,
                "transition" => (h - 0.5) * 140.0,
                _ => (h - 0.5) * 24.0,
            };
            let ctoken = format!("c{}", s.currency_code);
            SpokeMock {
                rank: i + 1,
                flag: s.flag.clone(),
                name: s.name.clone(),
                code: s.code.clone(),
                slug: s.slug.clone(),
                region: s.region.clone(),
                currency: s.currency_code.clone(),
                ctoken,
                float_eth,
                alpha,
                regime,
                window,
                hub,
                basis_bp,
                cap_b_usd: s.money_supply_b_usd,
            }
        })
        .collect();

    let float_eth: f64 = spokes.iter().map(|s| s.float_eth).sum();
    // Reserve = equity + (float / (1-h) * utilization padding) — mock solvent book.
    // R · (1-h) ≥ F  ⇒  R ≥ F/(1-h); add buffer so SCR ~ 1.3–1.5
    let min_r = float_eth / (1.0 - MOCK_H);
    let reserve_eth = (min_r * (1.0 + MOCK_BETA) + equity * 0.15).max(equity);
    // Redemption cost ≈ F / (1-h) * (1 + own-impact mock 0.08)
    let redemption = (float_eth / (1.0 - MOCK_H)) * 1.08;
    let scr = if redemption > 0.0 {
        reserve_eth / redemption
    } else {
        99.0
    };

    // Honesty metrics — mock healthy bank.
    let pi_true = 42.3 + (hash01("pi") - 0.5) * 8.0;
    let fcf = 18.1 + (hash01("fcf") - 0.5) * 4.0;
    let basis_bp = spokes
        .iter()
        .map(|s| s.basis_bp.abs() * s.float_eth)
        .sum::<f64>()
        / float_eth.max(1.0)
        * if hash01("basis") > 0.5 { 1.0 } else { -1.0 };

    let windows = spokes.iter().filter(|s| s.window).count();
    let hubs = spokes.iter().filter(|s| s.hub).count();

    // Cap display to top spokes for dashboard table (full list still earth board).
    spokes.truncate(48);

    BankMock {
        equity_eth: equity,
        reserve_eth,
        haircut: MOCK_H,
        beta: MOCK_BETA,
        f_max_eth: f_max,
        float_eth,
        scr,
        pi_true_eth: pi_true,
        fcf_eth: fcf,
        basis_bp,
        windows,
        hubs,
        spokes,
    }
}

fn scr_cls(scr: f64) -> &'static str {
    if scr >= 1.0 + MOCK_BETA {
        "kpi ok"
    } else if scr >= 1.0 {
        "kpi warn"
    } else {
        "kpi bad"
    }
}

fn regime_cls(r: &str) -> &'static str {
    match r {
        "taker" => "regime taker",
        "maker" => "regime maker",
        _ => "regime transition",
    }
}

#[component]
pub fn StatesPage() -> impl IntoView {
    let file = load_earth();
    let all = file.states;
    let total_count = file.count;
    let bank = build_bank_mock(&all);
    let regions = {
        let mut r: Vec<String> = all.iter().map(|s| s.region.clone()).collect();
        r.sort();
        r.dedup();
        r
    };
    let rows = RwSignal::new(all);
    let sort = RwSignal::new(SortKey::CapDesc);
    let region_f = RwSignal::new(String::new());
    let q = RwSignal::new(String::new());
    let only_capped = RwSignal::new(false);
    let board = RwSignal::new(Board::Bank);

    Effect::new(move |_| {
        document().set_title("Cyberia — states · bank above banks");
    });

    let filtered = move || {
        let rf = region_f.get();
        let query = q.get().trim().to_lowercase();
        let capped = only_capped.get();
        let mut list: Vec<EarthState> = rows
            .get()
            .into_iter()
            .filter(|s| rf.is_empty() || s.region == rf)
            .filter(|s| !capped || s.money_supply_b_usd > 0.0)
            .filter(|s| {
                if query.is_empty() {
                    true
                } else {
                    s.name.to_lowercase().contains(&query)
                        || s.code.to_lowercase().contains(&query)
                        || s.slug.to_lowercase().contains(&query)
                        || s.currency_code.to_lowercase().contains(&query)
                        || s.region.to_lowercase().contains(&query)
                }
            })
            .collect();
        match sort.get() {
            SortKey::CapDesc | SortKey::FloatDesc => list.sort_by(|a, b| {
                b.money_supply_b_usd
                    .partial_cmp(&a.money_supply_b_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::CapAsc => list.sort_by(|a, b| {
                a.money_supply_b_usd
                    .partial_cmp(&b.money_supply_b_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::Name => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            SortKey::Region => list.sort_by(|a, b| {
                a.region.cmp(&b.region).then_with(|| {
                    b.money_supply_b_usd
                        .partial_cmp(&a.money_supply_b_usd)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            }),
            SortKey::PopDesc => list.sort_by(|a, b| b.population.cmp(&a.population)),
        }
        list
    };

    let n_show = move || filtered().len();
    let total_cap = move || filtered().iter().map(|s| s.money_supply_b_usd).sum::<f64>();

    let bank_for_view = bank.clone();
    let bank_for_dock = bank.clone();
    let bank_spokes = bank.spokes.clone();
    let bank_f_max = bank.f_max_eth;
    let bank_float = bank.float_eth;
    let util_pct = (bank.float_eth / bank.f_max_eth.max(1.0) * 100.0).clamp(0.0, 100.0);

    view! {
        <div class="page-shell cities-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href="/cities" class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <div class="cyberia-phase-pill">
                            <span class="phase-dot"></span>
                            {move || match board.get() {
                                Board::Bank => "BANK · MOCK · SCR".to_string(),
                                Board::Earth => format!("{} EARTH · BY CAP", n_show()),
                            }}
                        </div>
                        <CyberiaNav active="states" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"PROTOCOL · BANK ABOVE BANKS"</div>
                        <h2 class="cities-title">"States"</h2>
                        <p class="cities-lead">
                            "Earth capital ranks from cyberstates, plus a mock solvent synthetic-FX bank — one ETH reserve, escaped float, SCR. Mock numbers only; paper lives in cyberia protocol."
                        </p>
                    </div>
                </div>

                // board switch
                <div class="list-toolbar bank-toolbar">
                    <div class="list-filters">
                        <button
                            class=move || if board.get() == Board::Bank { "chip chip-on" } else { "chip" }
                            on:click=move |_| board.set(Board::Bank)
                        >"BANK DASHBOARD"</button>
                        <button
                            class=move || if board.get() == Board::Earth { "chip chip-on" } else { "chip" }
                            on:click=move |_| board.set(Board::Earth)
                        >"EARTH BY CAP"</button>
                    </div>
                    <span class="bank-mock-tag">"MOCK · soft3 local · not live"</span>
                </div>

                // ── BANK BOARD ──
                {move || (board.get() == Board::Bank).then(|| {
                    let b = bank_for_view.clone();
                    let spokes = bank_spokes.clone();
                    let util = util_pct;
                    let f_max = bank_f_max;
                    let f_used = bank_float;
                    view! {
                        <div class="bank-dash">
                            <div class="bank-invariant">
                                <span class="bank-inv-k">"INVARIANT"</span>
                                <code class="bank-inv-eq">"Σ Fᵢ·Pᵢ  ≤  R_ETH · (1 − h)"</code>
                                <span class="bank-inv-sub">
                                    {format!(
                                        "h={:.0}% · β={:.0}% · F_max = E/(h+β) ≈ {:.1}× equity",
                                        b.haircut * 100.0,
                                        b.beta * 100.0,
                                        1.0 / (b.haircut + b.beta)
                                    )}
                                </span>
                            </div>

                            <div class="bank-kpi-grid">
                                <div class=scr_cls(b.scr)>
                                    <div class="kpi-lab">"SCR"</div>
                                    <div class="kpi-val">{format!("{:.2}", b.scr)}</div>
                                    <div class="kpi-sub">{format!("hold ≥ {:.2}", 1.0 + b.beta)}</div>
                                </div>
                                <div class="kpi">
                                    <div class="kpi-lab">"RESERVE R"</div>
                                    <div class="kpi-val">{format!("{} ETH", fmt_eth(b.reserve_eth))}</div>
                                    <div class="kpi-sub">{format!("≈ ${:.1}M", b.reserve_eth * MOCK_ETH_USD / 1e6)}</div>
                                </div>
                                <div class="kpi">
                                    <div class="kpi-lab">"EQUITY E"</div>
                                    <div class="kpi-val">{format!("{} ETH", fmt_eth(b.equity_eth))}</div>
                                    <div class="kpi-sub">"first-loss · rate limiter"</div>
                                </div>
                                <div class="kpi">
                                    <div class="kpi-lab">"FLOAT F"</div>
                                    <div class="kpi-val">{format!("{} ETH", fmt_eth(b.float_eth))}</div>
                                    <div class="kpi-sub">{format!("{:.0}% of F_max {}", util, fmt_eth(b.f_max_eth))}</div>
                                </div>
                                <div class="kpi">
                                    <div class="kpi-lab">"Π_TRUE"</div>
                                    <div class="kpi-val pos">{format!("+{} ETH", fmt_eth(b.pi_true_eth))}</div>
                                    <div class="kpi-sub">"ΔR − ΔF·(1−h) · week"</div>
                                </div>
                                <div class="kpi">
                                    <div class="kpi-lab">"TOPOLOGY"</div>
                                    <div class="kpi-val">{format!("{}w · {}h", b.windows, b.hubs)}</div>
                                    <div class="kpi-sub">"windows · hubs · from flow"</div>
                                </div>
                            </div>

                            <div class="bank-util">
                                <div class="bank-util-lab">
                                    <span>"FLOAT CAPACITY"</span>
                                    <span>{format!("{} / {} ETH", fmt_eth(f_used), fmt_eth(f_max))}</span>
                                </div>
                                <div class="bank-util-track">
                                    <div class="bank-util-fill" style=format!("width:{util:.1}%")></div>
                                </div>
                            </div>

                            // Honesty dashboard §8.1
                            <div class="bank-honesty">
                                <div class="bank-section-h">"HONESTY · FOUR NUMBERS"</div>
                                <div class="honesty-grid">
                                    <div class="honesty-card">
                                        <div class="hon-n">"1"</div>
                                        <div class="hon-lab">"REALIZED ETH FCF"</div>
                                        <div class="hon-val pos">{format!("+{} ETH", fmt_eth(b.fcf_eth))}</div>
                                        <div class="hon-why">"detects mark-to-model illusion"</div>
                                    </div>
                                    <div class="honesty-card">
                                        <div class="hon-n">"2"</div>
                                        <div class="hon-lab">"Π_TRUE"</div>
                                        <div class="hon-val pos">{format!("+{} ETH", fmt_eth(b.pi_true_eth))}</div>
                                        <div class="hon-why">"profit that is not float expansion"</div>
                                    </div>
                                    <div class="honesty-card">
                                        <div class="hon-n">"3"</div>
                                        <div class="hon-lab">"BASIS · FLOW-SYM"</div>
                                        <div class="hon-val">{format!("{:+.0} bp", b.basis_bp)}</div>
                                        <div class="hon-why">"external price vs model, two-sided"</div>
                                    </div>
                                    <div class="honesty-card">
                                        <div class="hon-n">"4"</div>
                                        <div class="hon-lab">"SCR"</div>
                                        <div class="hon-val">{format!("{:.2}", b.scr)}</div>
                                        <div class="hon-why">"slow-motion insolvency meter"</div>
                                    </div>
                                </div>
                            </div>

                            <div class="bank-section-h" style="margin-top: 18px;">
                                "SPOKES · ESCAPED FLOAT (TOP 48 BY CAP WEIGHT)"
                            </div>
                            <div class="states-table-wrap bank-table">
                                <div class="states-table-h bank-table-h">
                                    <span class="st-rank">"#"</span>
                                    <span class="st-name">"STATE"</span>
                                    <span class="st-token">"cTOKEN"</span>
                                    <span class="st-cap">"F·P ETH"</span>
                                    <span class="st-delta">"α"</span>
                                    <span class="st-pop">"REGIME"</span>
                                    <span class="st-region">"WINDOW"</span>
                                </div>
                                <div class="states-table-body bank-table-body">
                                    {spokes.into_iter().map(|s| {
                                        let href = format!("https://cyberstates.net/state/{}", s.slug);
                                        let float_s = fmt_eth(s.float_eth);
                                        let alpha_s = format!("{:.1}%", s.alpha * 100.0);
                                        let win = if s.window { "OPEN" } else { "—" };
                                        let hub_mark = if s.hub { " · HUB" } else { "" };
                                        let reg = s.regime.to_uppercase();
                                        let rcls = regime_cls(s.regime);
                                        view! {
                                            <a class="states-row bank-row" href=href target="_blank" rel="noopener">
                                                <span class="st-rank">{s.rank}</span>
                                                <span class="st-name">
                                                    <span class="st-flag">{s.flag}</span>
                                                    <span class="st-name-text">{s.name}</span>
                                                    <span class="st-code">{s.code}</span>
                                                </span>
                                                <span class="st-token">{s.ctoken}</span>
                                                <span class="st-cap">{float_s}</span>
                                                <span class="st-delta">{alpha_s}</span>
                                                <span class="st-pop">
                                                    <span class=rcls>{reg}</span>
                                                    <span class="hub-mark">{hub_mark}</span>
                                                </span>
                                                <span class=if s.window { "st-region win-open" } else { "st-region" }>{win}</span>
                                            </a>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>

                            <p class="bank-footnote">
                                "Mock dashboard of "
                                <em>"A Bank Above Banks"</em>
                                " — five axioms, one ETH reserve, escaped-float liabilities, SCR ≥ 1+β. Equity is the rate limiter: F_max = E/(h+β). Not live; no real mint or reserve."
                            </p>
                        </div>
                    }
                })}

                // ── EARTH BOARD ──
                {move || (board.get() == Board::Earth).then(|| view! {
                    <div class="list-toolbar">
                        <input
                            class="found-input list-search"
                            type="search"
                            placeholder="search name · code · token · region"
                            prop:value=move || q.get()
                            on:input=move |ev| q.set(event_target_value(&ev))
                        />
                        <div class="list-filters">
                            <button
                                class=move || if region_f.get().is_empty() { "chip chip-on" } else { "chip" }
                                on:click=move |_| region_f.set(String::new())
                            >"ALL REGIONS"</button>
                            {regions.clone().into_iter().map(|r| {
                                let r2 = r.clone();
                                let r3 = r.clone();
                                view! {
                                    <button
                                        class=move || if region_f.get() == r2 { "chip chip-on" } else { "chip" }
                                        on:click=move |_| region_f.set(r3.clone())
                                    >{r.to_uppercase()}</button>
                                }
                            }).collect_view()}
                            <button
                                class=move || if only_capped.get() { "chip chip-on" } else { "chip" }
                                on:click=move |_| only_capped.update(|v| *v = !*v)
                            >"CAPPED ONLY"</button>
                        </div>
                        <div class="list-sorts">
                            <span class="list-sort-label">"SORT"</span>
                            <button
                                class=move || if sort.get() == SortKey::CapDesc { "chip chip-on" } else { "chip" }
                                on:click=move |_| sort.set(SortKey::CapDesc)
                            >"CAP ↓"</button>
                            <button
                                class=move || if sort.get() == SortKey::CapAsc { "chip chip-on" } else { "chip" }
                                on:click=move |_| sort.set(SortKey::CapAsc)
                            >"CAP ↑"</button>
                            <button
                                class=move || if sort.get() == SortKey::PopDesc { "chip chip-on" } else { "chip" }
                                on:click=move |_| sort.set(SortKey::PopDesc)
                            >"POP ↓"</button>
                            <button
                                class=move || if sort.get() == SortKey::Name { "chip chip-on" } else { "chip" }
                                on:click=move |_| sort.set(SortKey::Name)
                            >"NAME"</button>
                            <button
                                class=move || if sort.get() == SortKey::Region { "chip chip-on" } else { "chip" }
                                on:click=move |_| sort.set(SortKey::Region)
                            >"REGION"</button>
                        </div>
                    </div>

                    <div class="states-table-wrap">
                        <div class="states-table-h">
                            <span class="st-rank">"#"</span>
                            <span class="st-name">"STATE"</span>
                            <span class="st-token">"TOKEN"</span>
                            <span class="st-cap">"CAPITAL"</span>
                            <span class="st-delta">"Δ"</span>
                            <span class="st-pop">"POP"</span>
                            <span class="st-region">"REGION"</span>
                        </div>
                        <div class="states-table-body">
                            {move || filtered().into_iter().enumerate().map(|(i, s)| {
                                let rank = i + 1;
                                let href = format!("https://cyberstates.net/state/{}", s.slug);
                                let cap = fmt_cap(s.money_supply_b_usd);
                                let cap_soon = s.money_supply_b_usd <= 0.0;
                                let delta = cap_delta_pct(s.money_supply_b_usd, s.money_supply_b_usd_prev);
                                let pop = fmt_pop(s.population);
                                let name = s.name.clone();
                                let flag = s.flag.clone();
                                let code = s.code.clone();
                                let token = s.currency_code.clone();
                                let region = s.region.clone();
                                let podium = match rank {
                                    1 => " podium-1",
                                    2 => " podium-2",
                                    3 => " podium-3",
                                    _ => "",
                                };
                                view! {
                                    <a class=format!("states-row{podium}") href=href target="_blank" rel="noopener">
                                        <span class="st-rank" style=format!(
                                            "color: {}; font-weight: {};",
                                            if rank <= 3 { "var(--cyber-yellow)" } else if rank <= 10 { "var(--cyber-cyan)" } else { "#666" },
                                            if rank <= 10 { "700" } else { "400" },
                                        )>{rank}</span>
                                        <span class="st-name">
                                            <span class="st-flag">{flag}</span>
                                            <span class="st-name-text">{name}</span>
                                            <span class="st-code">{code}</span>
                                        </span>
                                        <span class="st-token">{token}</span>
                                        <span class=if cap_soon { "st-cap soon" } else { "st-cap" }>{cap}</span>
                                        <span class="st-delta">
                                            {match delta {
                                                Some(d) => {
                                                    let (t, c) = fmt_delta(d);
                                                    view! { <span style:color=c>{t}</span> }.into_any()
                                                }
                                                None => view! { <span style="color:#333;">"—"</span> }.into_any(),
                                            }}
                                        </span>
                                        <span class="st-pop">{pop}</span>
                                        <span class="st-region">{region}</span>
                                    </a>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                })}
            </div>

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || match board.get() {
                        Board::Bank => format!(
                            "SCR {:.2} · R {} ETH · F {} ETH · mock bank",
                            bank_for_dock.scr,
                            fmt_eth(bank_for_dock.reserve_eth),
                            fmt_eth(bank_for_dock.float_eth),
                        ),
                        Board::Earth => {
                            let n = n_show();
                            let cap = total_cap();
                            format!("{n} / {total_count} earth · Σ capital {} · cyberstates", fmt_cap(cap))
                        }
                    }}
                </span>
                <a
                    class="cta-btn cta-found cta-lg dock-found"
                    href="https://cyberstates.net"
                    target="_blank"
                    rel="noopener"
                    style="text-decoration:none; max-width: 320px;"
                >
                    <span class="cta-copy">
                        <span class="cta-title">"OPEN TERMINAL"</span>
                        <span class="cta-sub">"cyberstates.net"</span>
                    </span>
                </a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
