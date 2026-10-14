//! `/docs` — the document library of the leasehold: the pack, the rules the
//! city votes on, the index and the rates, and the land and company papers
//! a Holder receives before signing. Graph pages link out; files live on
//! IPFS by their content hash.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::orders::support_href;
use leptos::prelude::*;
use serde::Deserialize;

const LIBRARY_JSON: &str = include_str!("docs_library.json");
pub const GATEWAY: &str = "https://gateway.pinata.cloud/ipfs/";

#[derive(Clone, Debug, Deserialize)]
pub struct Section {
    pub section: String,
    #[serde(default)]
    pub line: String,
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
    #[serde(default)]
    pub note: String,
}

impl Item {
    /// Where the document opens: a graph page, or its IPFS address.
    pub fn href(&self) -> Option<String> {
        if !self.url.is_empty() {
            Some(self.url.clone())
        } else if !self.cid.is_empty() {
            Some(format!("{GATEWAY}{}", self.cid))
        } else {
            None
        }
    }

    pub fn meta(&self) -> String {
        if !self.cid.is_empty() {
            format!("ipfs · {}…", &self.cid[..self.cid.len().min(12)])
        } else if !self.url.is_empty() {
            self.url
                .trim_start_matches("https://")
                .trim_end_matches('/')
                .to_string()
        } else if !self.note.is_empty() {
            self.note.clone()
        } else {
            "pinned to IPFS before signing · from support until then".to_string()
        }
    }
}

pub fn library() -> &'static [Section] {
    static LIB: std::sync::OnceLock<Vec<Section>> = std::sync::OnceLock::new();
    LIB.get_or_init(|| serde_json::from_str(LIBRARY_JSON).unwrap_or_default())
}

#[component]
pub fn DocsPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — leasehold documents");
    });
    let support = support_href();
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
            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker">"LEASEHOLD · DOCUMENTS"</div>
                        <h2 class="cities-title">"Library"</h2>
                        <p class="cities-lead">
                            "Everything a Holder reads and receives: the pack, the rules the city votes on, the index, and the land and company papers of the deal. Files live on IPFS by their content hash; pages live in the graph."
                        </p>
                    </div>
                    <a class="cta-btn cta-lease dock-found" href="/cabinet" style="text-decoration:none; max-width: 220px;">
                        <span class="cta-copy">
                            <span class="cta-title">"← CABINET"</span>
                            <span class="cta-sub">"your statuses"</span>
                        </span>
                    </a>
                </div>
                <div class="library">
                    {library().iter().map(|s| view! {
                        <section class="plot-card">
                            <div class="cyberia-panel-h">
                                <span class="panel-kicker">{s.section.to_uppercase()}</span>
                                <span class="panel-sub">{s.line.clone()}</span>
                            </div>
                            <div class="docs-list">
                                {s.items.iter().map(|it| {
                                    let meta = it.meta();
                                    let support = support.clone();
                                    match it.href() {
                                        Some(href) => view! {
                                            <div class="docs-row">
                                                <a class="doc-dl" href=href target="_blank" rel="noopener">
                                                    <span class="doc-name">{it.title.clone()}</span>
                                                    <span class="doc-meta">{it.what.clone()}</span>
                                                    <span class="doc-meta doc-where">{meta}</span>
                                                </a>
                                            </div>
                                        }.into_any(),
                                        None => view! {
                                            <div class="docs-row">
                                                <a class="doc-dl doc-pending" href=support target="_blank" rel="noopener">
                                                    <span class="doc-name">{it.title.clone()}</span>
                                                    <span class="doc-meta">{it.what.clone()}</span>
                                                    <span class="doc-meta doc-where">{meta}</span>
                                                </a>
                                            </div>
                                        }.into_any(),
                                    }
                                }).collect_view()}
                            </div>
                        </section>
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
