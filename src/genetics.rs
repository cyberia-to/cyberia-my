//! Genetics — the living layer of the valley. Species are their own
//! fundamental entity: not elements, not products, but the genomes that
//! *produce* them. Each species enters the graph as a `species` word;
//! SEED writes one signed signal: species —produces→ coin goods (and the
//! word is then linkable: plot —grows→ species).

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{
    emit_signal, find_word, links_touching, mint_word, neuron, word_particle, Link,
};
use leptos::prelude::*;

pub struct SpeciesDef {
    /// latin binomial — the canonical word name
    pub latin: &'static str,
    pub common: &'static str,
    /// tree | crop | herb | animal | bird | insect | fungus
    pub category: &'static str,
    /// what it yields — each becomes a coin word on seed
    pub products: &'static [&'static str],
    pub blurb: &'static str,
    /// price of one living unit, CX
    pub price: f64,
    /// what one unit is — seedling · head · colony · crown · culm …
    pub unit: &'static str,
}

pub const SPECIES: &[SpeciesDef] = &[
    // —— trees ——
    SpeciesDef {
        latin: "Musa acuminata",
        common: "banana",
        category: "tree",
        products: &["banana", "banana-leaf"],
        blurb: "The valley staple — fruit in bunches, leaves as plates and wrap.",
        price: 2.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Persea americana",
        common: "avocado",
        category: "tree",
        products: &["avocado"],
        blurb: "Fat of the land — slow tree, rich yield.",
        price: 5.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Coffea arabica",
        common: "arabica coffee",
        category: "tree",
        products: &["coffee-cherry", "coffee"],
        blurb: "Highland cash crop — cherry to bean to cup.",
        price: 3.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Theobroma cacao",
        common: "cacao",
        category: "tree",
        products: &["cacao-pod", "cacao"],
        blurb: "Food of the gods — pod, nib, chocolate.",
        price: 4.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Cocos nucifera",
        common: "coconut palm",
        category: "tree",
        products: &["coconut", "copra", "coco-fiber"],
        blurb: "The tree of a thousand uses — water, flesh, oil, fiber, timber.",
        price: 3.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Mangifera indica",
        common: "mango",
        category: "tree",
        products: &["mango"],
        blurb: "Canopy fruit — shade above, sugar below.",
        price: 4.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Carica papaya",
        common: "papaya",
        category: "tree",
        products: &["papaya"],
        blurb: "Fast fruiter — months from seed to harvest.",
        price: 1.0,
        unit: "seedling",
    },
    SpeciesDef {
        latin: "Bambusa vulgaris",
        common: "bamboo",
        category: "tree",
        products: &["bamboo-pole", "bamboo-shoot"],
        blurb: "The building material that grows itself — a meter a day.",
        price: 1.5,
        unit: "culm",
    },
    // —— crops & herbs ——
    SpeciesDef {
        latin: "Oryza sativa",
        common: "rice",
        category: "crop",
        products: &["rice"],
        blurb: "The terrace grain — water, mud, sun.",
        price: 2.0,
        unit: "kg seed",
    },
    SpeciesDef {
        latin: "Ananas comosus",
        common: "pineapple",
        category: "crop",
        products: &["pineapple"],
        blurb: "Ground bromeliad — sweet armor.",
        price: 0.5,
        unit: "crown",
    },
    SpeciesDef {
        latin: "Zingiber officinale",
        common: "ginger",
        category: "herb",
        products: &["ginger"],
        blurb: "Rhizome heat — kitchen and medicine both.",
        price: 1.0,
        unit: "rhizome",
    },
    SpeciesDef {
        latin: "Curcuma longa",
        common: "turmeric",
        category: "herb",
        products: &["turmeric"],
        blurb: "Golden root — jamu's backbone.",
        price: 1.0,
        unit: "rhizome",
    },
    SpeciesDef {
        latin: "Vanilla planifolia",
        common: "vanilla",
        category: "herb",
        products: &["vanilla-pod"],
        blurb: "The orchid you hand-pollinate — patience priced by the gram.",
        price: 8.0,
        unit: "cutting",
    },
    // —— animals ——
    SpeciesDef {
        latin: "Ovis aries",
        common: "sheep",
        category: "animal",
        products: &["wool", "sheep-milk", "mutton"],
        blurb: "The flock — wool, milk, meat, and mowed grass.",
        price: 90.0,
        unit: "head",
    },
    SpeciesDef {
        latin: "Capra hircus",
        common: "goat",
        category: "animal",
        products: &["goat-milk", "goat-meat"],
        blurb: "The browser — thrives where lawns fail.",
        price: 70.0,
        unit: "head",
    },
    // —— birds ——
    SpeciesDef {
        latin: "Gallus gallus domesticus",
        common: "chicken",
        category: "bird",
        products: &["egg", "chicken-meat"],
        blurb: "The daily layer — eggs in, scraps out.",
        price: 8.0,
        unit: "head",
    },
    SpeciesDef {
        latin: "Anas platyrhynchos domesticus",
        common: "duck",
        category: "bird",
        products: &["duck-egg", "duck-meat"],
        blurb: "The paddy patrol — eats the pests rice can't.",
        price: 10.0,
        unit: "head",
    },
    // —— insects & fungi ——
    SpeciesDef {
        latin: "Apis cerana",
        common: "eastern honey bee",
        category: "insect",
        products: &["honey", "beeswax"],
        blurb: "The pollination engine — every orchard's silent partner.",
        price: 60.0,
        unit: "colony",
    },
    SpeciesDef {
        latin: "Pleurotus ostreatus",
        common: "oyster mushroom",
        category: "fungus",
        products: &["mushroom"],
        blurb: "Grows on what everything else throws away.",
        price: 2.0,
        unit: "kg spawn",
    },
];

pub const CATEGORIES: &[&str] = &["tree", "crop", "herb", "animal", "bird", "insect", "fungus"];

fn category_color(cat: &str) -> &'static str {
    match cat {
        "tree" => "var(--cyber-green)",
        "crop" => "var(--cyber-yellow)",
        "herb" => "var(--cyber-cyan)",
        "animal" => "var(--cyber-orange)",
        "bird" => "var(--cyber-magenta, #d36ee0)",
        "insect" => "var(--cyber-red)",
        _ => "#9a86e0",
    }
}

/// Has this species been seeded into the graph already?
fn seeded(latin: &str) -> bool {
    find_word(&word_particle("species", latin)).is_some()
}

/// Buy one living unit: CX out, stock in (id = common name), one signed
/// signal you —buys→ species. Seeds the genome first if it isn't yet.
fn buy_species(def: &SpeciesDef) -> Result<String, String> {
    use crate::wallet::{debit_cx, load_balance, load_profile, push_intent, stock_add};
    if load_balance().cx + 1e-9 < def.price {
        return Err(format!(
            "{:.1} CX needed — you hold {:.1}",
            def.price,
            load_balance().cx
        ));
    }
    if !seeded(def.latin) {
        seed_species(def)?;
    }
    let me = neuron().bech32;
    debit_cx(def.price);
    stock_add(def.common, 1.0);
    let handle = load_profile().handle;
    let you = mint_word("person", &handle, "YOU", &me, false);
    let sp = word_particle("species", def.latin);
    let rel = mint_word("relation", "buys", "market acquisition", "", true);
    emit_signal(
        vec![Link {
            from: you,
            rel,
            to: sp,
            weight: 1.0,
            note: format!("-{:.1} CX · 1 {}", def.price, def.unit),
        }],
        &format!("buy 1 {} {}", def.unit, def.common),
    )?;
    push_intent(&handle, "gen_buy", &format!("{} 1 {}", def.common, def.unit));
    Ok(format!(
        "bought 1 {} {} · -{:.1} CX",
        def.unit, def.common, def.price
    ))
}

/// Seed one species: mint its word + product coin words, and commit one
/// signal: species —produces→ each product. Idempotent per species.
fn seed_species(def: &SpeciesDef) -> Result<String, String> {
    let me = neuron().bech32;
    let sp = mint_word("species", def.latin, def.common, &me, false);
    let produces = mint_word(
        "relation",
        "produces",
        "genetic yield — species produces good",
        "",
        true,
    );
    let links: Vec<Link> = def
        .products
        .iter()
        .map(|p| Link {
            from: sp.clone(),
            rel: produces.clone(),
            to: mint_word("coin", p, "", &me, false),
            weight: 1.0,
            note: def.common.into(),
        })
        .collect();
    emit_signal(links, &format!("genome seed · {}", def.latin))?;
    Ok(sp)
}

#[component]
pub fn GeneticsPage() -> impl IntoView {
    let cat_filter = RwSignal::new(String::new());
    let tick = RwSignal::new(0u32);
    let msg = RwSignal::new(None::<(bool, String)>);

    Effect::new(move |_| {
        document().set_title("Cyberia — genetics");
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
                            {format!("GENOME · {} SPECIES", SPECIES.len())}
                        </div>
                        <CyberiaNav active="genetics" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"THE LIVING LAYER"</div>
                        <h2 class="cities-title">"Genetics"</h2>
                        <p class="cities-lead">
                            "Species are their own fundamental entity — not elements, not products, but the genomes that produce them. Seed one into the graph and it becomes a word: plots grow it, it produces coins, signals carry the assertions."
                        </p>
                    </div>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                <div class="list-filters" style="margin-bottom:14px;">
                    <button class=move || if cat_filter.get().is_empty() { "chip chip-on" } else { "chip" }
                        on:click=move |_| cat_filter.set(String::new())>"ALL"</button>
                    {CATEGORIES.iter().map(|c| {
                        let c2 = c.to_string();
                        let c3 = c.to_string();
                        view! {
                            <button class=move || if cat_filter.get() == c3 { "chip chip-on" } else { "chip" }
                                on:click=move |_| cat_filter.set(c2.clone())>{c.to_uppercase()}</button>
                        }
                    }).collect_view()}
                </div>

                <div class="studio-list">
                    {move || {
                        let _ = tick.get();
                        let f = cat_filter.get();
                        let list: Vec<&SpeciesDef> = SPECIES
                            .iter()
                            .filter(|s| f.is_empty() || s.category == f)
                            .collect();
                        view! {
                            {list.into_iter().map(|def| {
                                let particle = word_particle("species", def.latin);
                                let in_graph = seeded(def.latin);
                                let n_links = if in_graph { links_touching(&particle).len() } else { 0 };
                                let word_href = format!("/world/word/{particle}");
                                let grow_href = format!("/world/links/new?to={particle}");
                                let latin = def.latin;
                                let held = crate::wallet::stock_qty(def.common);
                                let seed = move |_| {
                                    match seed_species(def) {
                                        Ok(_) => msg.set(Some((true, format!("{latin} seeded — genome in the graph, produces links committed")))),
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                    tick.update(|n| *n += 1);
                                };
                                let buy = move |_| {
                                    match buy_species(def) {
                                        Ok(t) => msg.set(Some((true, t))),
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                    tick.update(|n| *n += 1);
                                };
                                view! {
                                    <div class="studio-row">
                                        <div class="studio-row-main">
                                            <span class="studio-kind" style=format!("color:{};border-color:{};", category_color(def.category), category_color(def.category))>
                                                {def.category.to_uppercase()}
                                            </span>
                                            <div>
                                                <div class="studio-title">
                                                    <span style="font-style: italic;">{def.latin}</span>
                                                    <span style="color:#667; margin-left:8px; font-style:normal;">{def.common}</span>
                                                </div>
                                                <div class="studio-meta">{def.blurb}</div>
                                                <div class="list-filters" style="margin-top:6px;">
                                                    <span style="color:#556; font-size:10px; letter-spacing:1px;">"PRODUCES"</span>
                                                    {def.products.iter().map(|p| view! {
                                                        <span class="chip" style="pointer-events:none;">{*p}</span>
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        </div>
                                        <div class="studio-row-acts">
                                            {(held > 0.0).then(|| view! {
                                                <span class="sys-tag">{format!("{held:.0} {}", def.unit)}</span>
                                            })}
                                            <button class="chip chip-on" on:click=buy>
                                                {format!("BUY · {:.1} CX/{}", def.price, def.unit)}
                                            </button>
                                            {if in_graph {
                                                view! {
                                                    <span class="sys-tag">{format!("IN GRAPH · {n_links}")}</span>
                                                    <a class="chip" href=word_href>"WORD →"</a>
                                                    <a class="chip" href=grow_href>"GROW ON PLOT"</a>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button class="chip" on:click=seed>"SEED → GRAPH"</button>
                                                }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        }
                    }}
                </div>

                <p class="bank-footnote">
                    "Seeding commits one signed signal per species: species —produces→ its coins. Then link plots to genomes: plot —grows→ species. The lexicon ranks what the valley actually lives on."
                </p>
            </div>
        </div>
    }
}
