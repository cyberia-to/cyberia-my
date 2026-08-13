//! Elements — fundamental chemistry. The periodic table with prices:
//! every element a coin, approximate bulk market price (≈ USD/kg, soft
//! CX 1:1), BUY/SELL straight into your stock. Synthetic elements with
//! no market show no price and don't trade. Derivatives live in
//! /products; this is the bottom shelf of matter.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{emit_signal, mint_word, neuron, Link};
use crate::wallet::{
    credit_cx, debit_cx, ensure_economy_boot, load_balance, push_intent, stock_add, stock_qty,
};
use leptos::prelude::*;

pub struct ElementDef {
    pub z: u8,
    pub sym: &'static str,
    pub name: &'static str,
    /// 1-18 main-table column; lanthanides/actinides get their own rows
    pub group: u8,
    pub period: u8,
    /// alkali | alkaline | transition | post | metalloid | nonmetal |
    /// halogen | noble | lanthanide | actinide
    pub cat: &'static str,
    /// ≈ bulk market price, USD/kg. None = synthetic / no market.
    pub price: Option<f64>,
}

macro_rules! el {
    ($z:expr, $sym:expr, $name:expr, $g:expr, $p:expr, $cat:expr, $price:expr) => {
        ElementDef {
            z: $z,
            sym: $sym,
            name: $name,
            group: $g,
            period: $p,
            cat: $cat,
            price: $price,
        }
    };
}

#[rustfmt::skip]
pub const ELEMENTS: &[ElementDef] = &[
    el!(1, "H", "hydrogen", 1, 1, "nonmetal", Some(1.4)),
    el!(2, "He", "helium", 18, 1, "noble", Some(24.0)),
    el!(3, "Li", "lithium", 1, 2, "alkali", Some(85.0)),
    el!(4, "Be", "beryllium", 2, 2, "alkaline", Some(850.0)),
    el!(5, "B", "boron", 13, 2, "metalloid", Some(3.7)),
    el!(6, "C", "carbon", 14, 2, "nonmetal", Some(0.12)),
    el!(7, "N", "nitrogen", 15, 2, "nonmetal", Some(0.14)),
    el!(8, "O", "oxygen", 16, 2, "nonmetal", Some(0.15)),
    el!(9, "F", "fluorine", 17, 2, "halogen", Some(2.2)),
    el!(10, "Ne", "neon", 18, 2, "noble", Some(240.0)),
    el!(11, "Na", "sodium", 1, 3, "alkali", Some(3.0)),
    el!(12, "Mg", "magnesium", 2, 3, "alkaline", Some(2.3)),
    el!(13, "Al", "aluminium", 13, 3, "post", Some(1.8)),
    el!(14, "Si", "silicon", 14, 3, "metalloid", Some(1.7)),
    el!(15, "P", "phosphorus", 15, 3, "nonmetal", Some(2.7)),
    el!(16, "S", "sulfur", 16, 3, "nonmetal", Some(0.1)),
    el!(17, "Cl", "chlorine", 17, 3, "halogen", Some(0.08)),
    el!(18, "Ar", "argon", 18, 3, "noble", Some(0.9)),
    el!(19, "K", "potassium", 1, 4, "alkali", Some(12.0)),
    el!(20, "Ca", "calcium", 2, 4, "alkaline", Some(2.3)),
    el!(21, "Sc", "scandium", 3, 4, "transition", Some(3400.0)),
    el!(22, "Ti", "titanium", 4, 4, "transition", Some(11.0)),
    el!(23, "V", "vanadium", 5, 4, "transition", Some(22.0)),
    el!(24, "Cr", "chromium", 6, 4, "transition", Some(9.4)),
    el!(25, "Mn", "manganese", 7, 4, "transition", Some(1.8)),
    el!(26, "Fe", "iron", 8, 4, "transition", Some(0.1)),
    el!(27, "Co", "cobalt", 9, 4, "transition", Some(33.0)),
    el!(28, "Ni", "nickel", 10, 4, "transition", Some(14.0)),
    el!(29, "Cu", "copper", 11, 4, "transition", Some(9.0)),
    el!(30, "Zn", "zinc", 12, 4, "transition", Some(2.5)),
    el!(31, "Ga", "gallium", 13, 4, "post", Some(150.0)),
    el!(32, "Ge", "germanium", 14, 4, "metalloid", Some(1000.0)),
    el!(33, "As", "arsenic", 15, 4, "metalloid", Some(1.0)),
    el!(34, "Se", "selenium", 16, 4, "nonmetal", Some(22.0)),
    el!(35, "Br", "bromine", 17, 4, "halogen", Some(4.4)),
    el!(36, "Kr", "krypton", 18, 4, "noble", Some(290.0)),
    el!(37, "Rb", "rubidium", 1, 5, "alkali", Some(15500.0)),
    el!(38, "Sr", "strontium", 2, 5, "alkaline", Some(6.6)),
    el!(39, "Y", "yttrium", 3, 5, "transition", Some(31.0)),
    el!(40, "Zr", "zirconium", 4, 5, "transition", Some(36.0)),
    el!(41, "Nb", "niobium", 5, 5, "transition", Some(73.0)),
    el!(42, "Mo", "molybdenum", 6, 5, "transition", Some(40.0)),
    el!(43, "Tc", "technetium", 7, 5, "transition", None),
    el!(44, "Ru", "ruthenium", 8, 5, "transition", Some(10500.0)),
    el!(45, "Rh", "rhodium", 9, 5, "transition", Some(170000.0)),
    el!(46, "Pd", "palladium", 10, 5, "transition", Some(35000.0)),
    el!(47, "Ag", "silver", 11, 5, "transition", Some(1500.0)),
    el!(48, "Cd", "cadmium", 12, 5, "transition", Some(2.7)),
    el!(49, "In", "indium", 13, 5, "post", Some(170.0)),
    el!(50, "Sn", "tin", 14, 5, "post", Some(30.0)),
    el!(51, "Sb", "antimony", 15, 5, "metalloid", Some(5.8)),
    el!(52, "Te", "tellurium", 16, 5, "metalloid", Some(63.0)),
    el!(53, "I", "iodine", 17, 5, "halogen", Some(35.0)),
    el!(54, "Xe", "xenon", 18, 5, "noble", Some(1800.0)),
    el!(55, "Cs", "caesium", 1, 6, "alkali", Some(61000.0)),
    el!(56, "Ba", "barium", 2, 6, "alkaline", Some(0.55)),
    el!(57, "La", "lanthanum", 3, 6, "lanthanide", Some(4.9)),
    el!(58, "Ce", "cerium", 4, 6, "lanthanide", Some(4.7)),
    el!(59, "Pr", "praseodymium", 5, 6, "lanthanide", Some(103.0)),
    el!(60, "Nd", "neodymium", 6, 6, "lanthanide", Some(57.0)),
    el!(61, "Pm", "promethium", 7, 6, "lanthanide", None),
    el!(62, "Sm", "samarium", 8, 6, "lanthanide", Some(13.9)),
    el!(63, "Eu", "europium", 9, 6, "lanthanide", Some(31.0)),
    el!(64, "Gd", "gadolinium", 10, 6, "lanthanide", Some(29.0)),
    el!(65, "Tb", "terbium", 11, 6, "lanthanide", Some(660.0)),
    el!(66, "Dy", "dysprosium", 12, 6, "lanthanide", Some(310.0)),
    el!(67, "Ho", "holmium", 13, 6, "lanthanide", Some(57.0)),
    el!(68, "Er", "erbium", 14, 6, "lanthanide", Some(26.0)),
    el!(69, "Tm", "thulium", 15, 6, "lanthanide", Some(3000.0)),
    el!(70, "Yb", "ytterbium", 16, 6, "lanthanide", Some(17.0)),
    el!(71, "Lu", "lutetium", 17, 6, "lanthanide", Some(640.0)),
    el!(72, "Hf", "hafnium", 4, 6, "transition", Some(900.0)),
    el!(73, "Ta", "tantalum", 5, 6, "transition", Some(300.0)),
    el!(74, "W", "tungsten", 6, 6, "transition", Some(30.0)),
    el!(75, "Re", "rhenium", 7, 6, "transition", Some(2900.0)),
    el!(76, "Os", "osmium", 8, 6, "transition", Some(12000.0)),
    el!(77, "Ir", "iridium", 9, 6, "transition", Some(150000.0)),
    el!(78, "Pt", "platinum", 10, 6, "transition", Some(32000.0)),
    el!(79, "Au", "gold", 11, 6, "transition", Some(130000.0)),
    el!(80, "Hg", "mercury", 12, 6, "transition", Some(30.0)),
    el!(81, "Tl", "thallium", 13, 6, "post", Some(4200.0)),
    el!(82, "Pb", "lead", 14, 6, "post", Some(2.0)),
    el!(83, "Bi", "bismuth", 15, 6, "post", Some(6.4)),
    el!(84, "Po", "polonium", 16, 6, "post", None),
    el!(85, "At", "astatine", 17, 6, "halogen", None),
    el!(86, "Rn", "radon", 18, 6, "noble", None),
    el!(87, "Fr", "francium", 1, 7, "alkali", None),
    el!(88, "Ra", "radium", 2, 7, "alkaline", None),
    el!(89, "Ac", "actinium", 3, 7, "actinide", None),
    el!(90, "Th", "thorium", 4, 7, "actinide", Some(290.0)),
    el!(91, "Pa", "protactinium", 5, 7, "actinide", None),
    el!(92, "U", "uranium", 6, 7, "actinide", Some(130.0)),
    el!(93, "Np", "neptunium", 7, 7, "actinide", None),
    el!(94, "Pu", "plutonium", 8, 7, "actinide", None),
    el!(95, "Am", "americium", 9, 7, "actinide", Some(750000.0)),
    el!(96, "Cm", "curium", 10, 7, "actinide", None),
    el!(97, "Bk", "berkelium", 11, 7, "actinide", None),
    el!(98, "Cf", "californium", 12, 7, "actinide", Some(27_000_000_000.0)),
    el!(99, "Es", "einsteinium", 13, 7, "actinide", None),
    el!(100, "Fm", "fermium", 14, 7, "actinide", None),
    el!(101, "Md", "mendelevium", 15, 7, "actinide", None),
    el!(102, "No", "nobelium", 16, 7, "actinide", None),
    el!(103, "Lr", "lawrencium", 17, 7, "actinide", None),
    el!(104, "Rf", "rutherfordium", 4, 7, "transition", None),
    el!(105, "Db", "dubnium", 5, 7, "transition", None),
    el!(106, "Sg", "seaborgium", 6, 7, "transition", None),
    el!(107, "Bh", "bohrium", 7, 7, "transition", None),
    el!(108, "Hs", "hassium", 8, 7, "transition", None),
    el!(109, "Mt", "meitnerium", 9, 7, "transition", None),
    el!(110, "Ds", "darmstadtium", 10, 7, "transition", None),
    el!(111, "Rg", "roentgenium", 11, 7, "transition", None),
    el!(112, "Cn", "copernicium", 12, 7, "transition", None),
    el!(113, "Nh", "nihonium", 13, 7, "post", None),
    el!(114, "Fl", "flerovium", 14, 7, "post", None),
    el!(115, "Mc", "moscovium", 15, 7, "post", None),
    el!(116, "Lv", "livermorium", 16, 7, "post", None),
    el!(117, "Ts", "tennessine", 17, 7, "halogen", None),
    el!(118, "Og", "oganesson", 18, 7, "noble", None),
];

pub fn cat_color(cat: &str) -> &'static str {
    match cat {
        "alkali" => "var(--cyber-red)",
        "alkaline" => "var(--cyber-orange)",
        "transition" => "var(--cyber-yellow)",
        "post" => "#9fb2c8",
        "metalloid" => "var(--cyber-cyan)",
        "nonmetal" => "var(--cyber-green)",
        "halogen" => "#7ee08a",
        "noble" => "#c98fff",
        "lanthanide" => "#ff8fc2",
        "actinide" => "#e0637a",
        _ => "#666",
    }
}

/// Compact price: 0.08 · 1.4 · 62k · 27B — table cells are small.
pub fn fmt_px(v: f64) -> String {
    if v >= 1e9 {
        format!("{:.0}B", v / 1e9)
    } else if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e3 {
        format!("{:.0}k", v / 1e3)
    } else if v >= 10.0 {
        format!("{v:.0}")
    } else if v >= 1.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// Grid placement: main table rows 1-7; La-Lu and Ac-Lr pulled into rows
/// 9/10 (row 8 stays empty as the visual gap), cols 3-17.
fn grid_pos(e: &ElementDef) -> (u8, u8) {
    if e.cat == "lanthanide" {
        (9, e.z - 57 + 3)
    } else if e.cat == "actinide" {
        (10, e.z - 89 + 3)
    } else {
        (e.period, e.group)
    }
}

/// One trade = one signed signal: you —buys/sells→ element word.
fn trade(e: &ElementDef, qty: f64, sell: bool) -> Result<String, String> {
    let Some(px) = e.price else {
        return Err(format!("{} — synthetic, no market", e.name));
    };
    if qty <= 0.0 {
        return Err("qty must be positive".into());
    }
    let total = px * qty;
    let me = neuron().bech32;
    let coin_w = mint_word(
        "coin",
        e.name,
        &format!("{} · Z={} · element", e.sym, e.z),
        &me,
        false,
    );
    let handle = crate::wallet::load_profile().handle;
    let you_w = mint_word("person", &handle, "YOU", &me, false);
    if sell {
        if stock_qty(e.name) + 1e-9 < qty {
            return Err(format!("hold {:.3} kg — not enough", stock_qty(e.name)));
        }
        stock_add(e.name, -qty);
        credit_cx(total, "elements", &format!("sell {qty} kg {}", e.name));
        let rel = mint_word("relation", "sells", "market disposal", "", true);
        emit_signal(
            vec![Link {
                from: you_w,
                rel,
                to: coin_w,
                weight: qty,
                note: format!("+{total:.2} CX @ {px}/kg"),
            }],
            &format!("sell {qty} kg {}", e.name),
        )?;
        push_intent(&handle, "el_sell", &format!("{} {qty}kg", e.sym));
        Ok(format!("sold {qty} kg {} · +{total:.2} CX", e.name))
    } else {
        if load_balance().cx < total {
            return Err(format!(
                "{total:.2} CX needed — you hold {:.2}",
                load_balance().cx
            ));
        }
        debit_cx(total, "elements", &format!("buy {qty} kg {}", e.name));
        stock_add(e.name, qty);
        let rel = mint_word("relation", "buys", "market acquisition", "", true);
        emit_signal(
            vec![Link {
                from: you_w,
                rel,
                to: coin_w,
                weight: qty,
                note: format!("-{total:.2} CX @ {px}/kg"),
            }],
            &format!("buy {qty} kg {}", e.name),
        )?;
        push_intent(&handle, "el_buy", &format!("{} {qty}kg", e.sym));
        Ok(format!("bought {qty} kg {} · -{total:.2} CX", e.name))
    }
}

#[component]
pub fn ElementsPage() -> impl IntoView {
    let selected = RwSignal::new(None::<u8>); // Z
    let qty = RwSignal::new("1".to_string());
    let msg = RwSignal::new(None::<(bool, String)>);
    let tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        document().set_title("Cyberia — elements · fundamental chemistry");
        ensure_economy_boot();
        crate::erp::ensure_erp_boot();
        crate::signal::ensure_graph_boot();
    });

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
                            {move || {
                                let _ = tick.get();
                                format!("118 ELEMENTS · {} CX", crate::economy::fmt_qty(load_balance().cx))
                            }}
                        </div>
                        <CyberiaNav active="elements" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"FUNDAMENTAL CHEMISTRY"</div>
                        <h2 class="cities-title">"Elements"</h2>
                        <p class="cities-lead">
                            "The bottom shelf of matter — every element a coin with a market. Prices ≈ bulk USD/kg (CX 1:1); synthetic elements have no market. Derivatives live in " <a href="/products" style="color: var(--cyber-green);">"Products"</a> "."
                        </p>
                    </div>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                // selected element trade panel
                {move || {
                    let _ = tick.get();
                    selected.get().and_then(|z| ELEMENTS.iter().find(|e| e.z == z)).map(|e| {
                        let color = cat_color(e.cat);
                        let held = stock_qty(e.name);
                        let buy = move |_| {
                            let q: f64 = qty.get().trim().parse().unwrap_or(1.0);
                            let e = ELEMENTS.iter().find(|e| Some(e.z) == selected.get()).unwrap();
                            msg.set(Some(match trade(e, q, false) {
                                Ok(t) => (true, t),
                                Err(t) => (false, t),
                            }));
                            tick.update(|n| *n += 1);
                        };
                        let sell = move |_| {
                            let q: f64 = qty.get().trim().parse().unwrap_or(1.0);
                            let e = ELEMENTS.iter().find(|e| Some(e.z) == selected.get()).unwrap();
                            msg.set(Some(match trade(e, q, true) {
                                Ok(t) => (true, t),
                                Err(t) => (false, t),
                            }));
                            tick.update(|n| *n += 1);
                        };
                        view! {
                            <div class="el-panel">
                                <div class="el-panel-id">
                                    <div class="el-panel-sym" style=format!("color:{color};")>{e.sym}</div>
                                    <div>
                                        <div class="studio-title" style="text-transform:capitalize;">{e.name}</div>
                                        <div class="studio-meta">{format!("Z={} · {}", e.z, e.cat)}</div>
                                    </div>
                                </div>
                                <div class="el-panel-trade">
                                    {match e.price {
                                        Some(px) => view! {
                                            <div class="el-panel-px">
                                                <span class="kpi-lab">"PRICE"</span>
                                                <span class="kpi-val" style="font-size:18px;">{format!("{} CX/kg", fmt_px(px))}</span>
                                            </div>
                                            <div class="el-panel-px">
                                                <span class="kpi-lab">"YOU HOLD"</span>
                                                <span class="kpi-val" style="font-size:18px;">{format!("{held:.3} kg")}</span>
                                            </div>
                                            <input class="found-input el-qty" type="text" prop:value=move || qty.get()
                                                on:input=move |ev| qty.set(event_target_value(&ev))
                                                placeholder="kg" />
                                            <button class="chip chip-on" on:click=buy>"BUY"</button>
                                            <button class="chip" on:click=sell>"SELL"</button>
                                        }.into_any(),
                                        None => view! {
                                            <div class="studio-meta">"synthetic — lab curiosity, no market"</div>
                                        }.into_any(),
                                    }}
                                </div>
                            </div>
                        }
                    })
                }}

                // the table
                <div class="ptable-wrap">
                    <div class="ptable">
                        {ELEMENTS.iter().map(|e| {
                            let (row, col) = grid_pos(e);
                            let color = cat_color(e.cat);
                            let z = e.z;
                            let has_px = e.price.is_some();
                            view! {
                                <button
                                    class=move || if selected.get() == Some(z) { "ptable-cell ptable-on" } else { "ptable-cell" }
                                    style=format!(
                                        "grid-row:{row}; grid-column:{col}; --el-c:{color}; {}",
                                        if has_px { "" } else { "opacity:0.35;" }
                                    )
                                    on:click=move |_| selected.set(Some(z))
                                >
                                    <span class="ptable-z">{e.z}</span>
                                    <span class="ptable-sym">{e.sym}</span>
                                    <span class="ptable-px">{e.price.map(fmt_px).unwrap_or_else(|| "—".into())}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>

                // legend
                <div class="list-filters" style="margin-top:14px;">
                    {[
                        ("alkali", "ALKALI"), ("alkaline", "ALKALINE"), ("transition", "TRANSITION"),
                        ("post", "POST-METAL"), ("metalloid", "METALLOID"), ("nonmetal", "NONMETAL"),
                        ("halogen", "HALOGEN"), ("noble", "NOBLE"), ("lanthanide", "LANTHANIDE"),
                        ("actinide", "ACTINIDE"),
                    ].iter().map(|(cat, label)| view! {
                        <span class="chip" style=format!("pointer-events:none; color:{0}; border-color:{0};", cat_color(cat))>{*label}</span>
                    }).collect_view()}
                </div>

                <p class="bank-footnote">
                    "Every trade is one signed signal: you —buys/sells→ element, weight = kg. Holdings sit in your stock ledger next to products; the conservation view tracks them. Prices are soft-market approximations, not an oracle."
                </p>
            </div>
        </div>
    }
}
