//! `/docs` — the leasehold's documents as a showcase: what you sign, the
//! land's dossier, the Company's dossier, the rules the city votes on, the
//! index and the rates. Files live on IPFS by their content hash and show
//! their first page; pages live in the graph.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::orders::support_href;
use leptos::prelude::*;
use serde::Deserialize;

const LIBRARY_JSON: &str = include_str!("docs_library.json");
pub const GATEWAY: &str = "https://gateway.pinata.cloud/ipfs/";

#[derive(Clone, Debug, Deserialize)]
pub struct Section {
    /// `sign` · `land` · `company` · `rules` · `index`
    #[serde(default)]
    pub key: String,
    pub section: String,
    #[serde(default)]
    pub line: String,
    /// the prysm hue of the dossier
    #[serde(default)]
    pub color: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    pub title: String,
    #[serde(default)]
    pub what: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub cid: String,
    /// the first page, pinned beside the file
    #[serde(default)]
    pub thumb: String,
    /// the same page served beside the site, instant
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub pages: u32,
    #[serde(default)]
    pub kb: u32,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub title_en: String,
    /// second languages on the shelf: `en`, `ru`, … → the book beside the Indonesian
    #[serde(default)]
    pub langs: std::collections::BTreeMap<String, LangFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LangFile {
    #[serde(default)]
    pub cid: String,
    #[serde(default)]
    pub pages: u32,
    #[serde(default)]
    pub kb: u32,
}

/// Second languages a reader may ask for, in the order shown; the first two are on the shelf today.
pub const LANGS: &[(&str, &str)] = &[
    ("en", "EN"), ("ru", "RU"), ("zh", "中文"), ("de", "DE"), ("fr", "FR"), ("es", "ES"), ("ja", "日本語"),
    ("ko", "한국어"), ("ar", "AR"), ("pt", "PT"), ("it", "IT"), ("tr", "TR"), ("uk", "UK"), ("hi", "HI"),
];

impl Item {
    /// Where the document opens: its IPFS address, or a graph page.
    pub fn href(&self) -> Option<String> {
        if !self.cid.is_empty() {
            Some(format!("{GATEWAY}{}", self.cid))
        } else if !self.url.is_empty() {
            Some(self.url.clone())
        } else {
            None
        }
    }

    pub fn meta(&self) -> String {
        if !self.cid.is_empty() {
            format!("ipfs · {}…", &self.cid[..self.cid.len().min(12)])
        } else if !self.url.is_empty() {
            self.url.trim_start_matches("https://").trim_end_matches('/').to_string()
        } else if !self.note.is_empty() {
            self.note.clone()
        } else {
            "arrives before signing".to_string()
        }
    }

    /// `30 pages · 204 KB · ID / EN`
    fn facts(&self) -> String {
        let mut f = Vec::new();
        if self.pages > 0 {
            f.push(format!("{} {}", self.pages, if self.pages == 1 { "page" } else { "pages" }));
        }
        if self.kb > 0 {
            f.push(if self.kb >= 1024 { format!("{:.1} MB", self.kb as f64 / 1024.0) } else { format!("{} KB", self.kb) });
        }
        if !self.lang.is_empty() {
            f.push(self.lang.clone());
        }
        f.join(" · ")
    }

    fn hash_chip(&self) -> String {
        let c = &self.cid;
        if c.len() > 14 {
            format!("{}…{}", &c[..8], &c[c.len() - 6..])
        } else {
            c.clone()
        }
    }
}

pub fn library() -> &'static [Section] {
    static LIB: std::sync::OnceLock<Vec<Section>> = std::sync::OnceLock::new();
    LIB.get_or_init(|| serde_json::from_str(LIBRARY_JSON).unwrap_or_default())
}

fn dossier(s: &Section, support: &str) -> impl IntoView {
    let color = if s.color.is_empty() { "#ffffff".to_string() } else { s.color.clone() };
    let pinned = s.items.iter().filter(|i| !i.cid.is_empty()).count();
    let pending = s.items.len() - pinned;
    let id = s.key.clone();
    view! {
        <section class="dossier" id=id style:--ink=color.clone()>
            <div class="dossier-head">
                <div>
                    <div class="dossier-kicker">{s.section.to_uppercase()}</div>
                    <div class="dossier-line">{s.line.clone()}</div>
                </div>
                <div class="dossier-count">
                    <span>{format!("{pinned} on IPFS")}</span>
                    {(pending > 0).then(|| view! { <span class="dossier-pending">{format!("{pending} before signing")}</span> })}
                </div>
            </div>
            <div class="cover-grid">
                {s.items.iter().map(|it| {
                    let facts = it.facts();
                    match it.href() {
                        Some(href) if !it.cid.is_empty() => {
                            // the cover's address changes with the file, so a browser never shows a stale page
                            let thumb = if !it.cover.is_empty() {
                                Some(format!("{}?v={}", it.cover, &it.cid[it.cid.len().saturating_sub(8)..]))
                            } else {
                                (!it.thumb.is_empty()).then(|| format!("{GATEWAY}{}", it.thumb))
                            };
                            view! {
                                <a class="cover-card" href=href target="_blank" rel="noopener">
                                    <div class="cover">
                                        {match thumb {
                                            Some(t) => view! { <img src=t loading="lazy" alt=it.title.clone() /> }.into_any(),
                                            None => view! { <div class="cover-blank"></div> }.into_any(),
                                        }}
                                        <span class="cover-open">"OPEN"</span>
                                    </div>
                                    <div class="cover-title">{it.title.clone()}</div>
                                    <div class="cover-what">{it.what.clone()}</div>
                                    <div class="cover-meta">
                                        <span>{facts}</span>
                                        <span class="hash" title=it.cid.clone()>{it.hash_chip()}</span>
                                    </div>
                                </a>
                                {(!it.key.is_empty() && s.key == "sign").then(|| lang_row(it, support))}
                            }.into_any()
                        }
                        _ => {
                            let href = it.href().unwrap_or_else(|| support.to_string());
                            view! {
                                <a class="cover-card is-pending" href=href target="_blank" rel="noopener">
                                    <div class="cover"><div class="cover-blank"></div><span class="cover-open">"BEFORE SIGNING"</span></div>
                                    <div class="cover-title">{it.title.clone()}</div>
                                    <div class="cover-what">{it.what.clone()}</div>
                                    <div class="cover-meta"><span>{it.meta()}</span></div>
                                </a>
                            }.into_any()
                        }
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

/// Bahasa Indonesia is the text; a second language opens beside it, or is asked for.
fn lang_row(it: &Item, support: &str) -> impl IntoView {
    let title = if it.title_en.is_empty() { it.title.clone() } else { it.title_en.clone() };
    let id_href = format!("{GATEWAY}{}", it.cid);
    view! {
        <div class="lang-row">
            <a class="lang is-on" href=id_href target="_blank" rel="noopener" title="Bahasa Indonesia — the text that binds">"ID"</a>
            {LANGS.iter().map(|(code, label)| {
                match it.langs.get(*code) {
                    Some(f) if !f.cid.is_empty() => view! {
                        <a class="lang is-ready" href=format!("{GATEWAY}{}", f.cid) target="_blank" rel="noopener" title=format!("Bahasa Indonesia beside {label} · {} pages", f.pages)>{format!("+{label}")}</a>
                    }.into_any(),
                    _ => {
                        let text = format!("GENERATE {title} — Bahasa Indonesia + {label}");
                        let href = format!("{}?text={}", support, String::from(js_sys::encode_uri_component(&text)));
                        view! {
                            <a class="lang is-ask" href=href target="_blank" rel="noopener" title=format!("ask for the book beside {label} — made on request")>{format!("+{label}")}</a>
                        }.into_any()
                    }
                }
            }).collect_view()}
        </div>
    }
}

fn chips(s: &Section) -> impl IntoView {
    let color = if s.color.is_empty() { "#ffffff".to_string() } else { s.color.clone() };
    let id = s.key.clone();
    view! {
        <section class="dossier chips-section" id=id style:--ink=color>
            <div class="dossier-head">
                <div>
                    <div class="dossier-kicker">{s.section.to_uppercase()}</div>
                    <div class="dossier-line">{s.line.clone()}</div>
                </div>
            </div>
            <div class="chips">
                {s.items.iter().map(|it| {
                    let href = it.href().unwrap_or_default();
                    view! {
                        <a class="chip" href=href target="_blank" rel="noopener">
                            <span class="chip-title">{it.title.clone()}</span>
                            <span class="chip-what">{it.what.clone()}</span>
                        </a>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
pub fn DocsPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — leasehold documents");
    });
    let support = support_href();
    let lib = library();
    let files: Vec<&Item> = lib.iter().filter(|s| s.key != "rules" && s.key != "index").flat_map(|s| s.items.iter()).collect();
    let on_ipfs = files.iter().filter(|i| !i.cid.is_empty()).count();
    let pending = files.len() - on_ipfs;
    let signable = lib.iter().find(|s| s.key == "sign").map(|s| s.items.len()).unwrap_or(0);
    let pages: u32 = files.iter().map(|i| i.pages).sum();

    view! {
        <div class="page-shell cities-shell plot-shell">
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
                        <CyberiaNav active="cabinet" />
                    </div>
                </div>
            </div>
            <div class="cities-stage plot-stage-wrap docs-stage">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker">"LEASEHOLD · DOCUMENTS"</div>
                        <h2 class="cities-title">"Everything you sign and receive"</h2>
                        <p class="cities-lead">
                            "The instruments on the Company's letterhead, the land's papers, the Company's registry — each file by its content hash, nothing on our servers."
                        </p>
                    </div>
                    <div class="docs-counters">
                        <div class="counter"><b>{signable}</b><span>"to sign"</span></div>
                        <div class="counter"><b>{on_ipfs}</b><span>"on IPFS"</span></div>
                        <div class="counter"><b>{pages}</b><span>"pages"</span></div>
                        {(pending > 0).then(|| view! { <div class="counter is-dim"><b>{pending}</b><span>"before signing"</span></div> })}
                    </div>
                </div>
                <nav class="docs-jump">
                    {lib.iter().map(|s| {
                        let href = format!("#{}", s.key);
                        let ink = s.color.clone();
                        view! { <a href=href style:--ink=ink>{s.section.split(" —").next().unwrap_or("").split(" ·").next().unwrap_or("").to_uppercase()}</a> }
                    }).collect_view()}
                </nav>
                {lib.iter().map(|s| if s.key == "rules" || s.key == "index" { chips(s).into_any() } else { dossier(s, &support).into_any() }).collect_view()}
            </div>
        </div>
    }
}
