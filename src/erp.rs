//! The cyberia dialect kernel — cards · coins · motifs (templates) ·
//! intents · schedules · views, all riding the signal graph (signal.rs).
//! PLUMB ops move real stock; every run lands in the graph as one signed
//! signal. cyberia/protocol/system.md is the dialect spec.

use crate::economy::BOMS;
use crate::wallet::{
    ensure_economy_boot, load_leases, load_profile, push_intent, stock_add, stock_consume,
    stock_has, stock_qty,
};
use serde::{Deserialize, Serialize};

pub const CARDS_KEY: &str = "cyberia_erp_cards";
pub const ERP_INTENTS_KEY: &str = "cyberia_erp_intents";
pub const TEMPLATES_KEY: &str = "cyberia_erp_templates";
pub const SCHEDULES_KEY: &str = "cyberia_erp_schedules";
pub const VIEWS_KEY: &str = "cyberia_erp_views";
pub const ERP_BOOT_KEY: &str = "cyberia_erp_boot";
pub const TPL_SEED_KEY: &str = "cyberia_erp_tpl_seed_v2";
pub const VIEW_SEED_KEY: &str = "cyberia_erp_view_seed_v3";

// ─── primitives ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    /// person | city | plot | place | building | project | asset
    pub kind: String,
    pub name: String,
    pub owner: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IoLine {
    pub id: String,
    pub qty: f64,
}

/// Declared recipe — construct or transform (system.md Template).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserTemplate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub blurb: String,
    /// construct | transform | custom
    pub kind: String,
    pub burns: Vec<IoLine>,
    pub mints_coin: Vec<IoLine>,
    /// if Some, mint building Card of this class under target plot
    #[serde(default)]
    pub mints_building: Option<String>,
    #[serde(default)]
    pub needs_plot: bool,
    /// system seed vs user-created
    #[serde(default)]
    pub system: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErpIntent {
    pub id: u64,
    pub owner: String,
    pub kind: String,
    pub template_id: String,
    /// draft → reserved → done | cancelled
    pub state: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub note: String,
    /// schedule that spawned this (if any)
    #[serde(default)]
    pub schedule_id: Option<String>,
}

/// Schedule fires a template → creates intent (+ optional auto-run).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub template_id: String,
    #[serde(default)]
    pub target: Option<String>,
    /// every_n_ticks — soft clock: user presses TICK or FIRE
    pub every_ticks: u32,
    #[serde(default)]
    pub tick_count: u32,
    #[serde(default)]
    pub enabled: bool,
    /// if true, run template when fired; else only draft intent
    #[serde(default)]
    pub auto_run: bool,
    #[serde(default)]
    pub note: String,
}

/// View — derived projection (system.md). Never mutates. Declaration only.
/// Opening a view materializes rows from ledger + cybergraph + intents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErpView {
    pub id: String,
    pub name: String,
    /// inventory | balance | kanban | graph | calendar | memory |
    /// balance_sheet | profit_loss | cash_flow | flow | custom
    pub kind: String,
    /// optional focus particle (card id, person-*, city-*, …)
    #[serde(default)]
    pub focus: Option<String>,
    /// free filter: class, rel, state, kind, substring
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub system: bool,
}

/// One projected row from materializing a view (read-only).
#[derive(Clone, Debug, PartialEq)]
pub struct ViewRow {
    pub key: String,
    pub tag: String,
    pub cells: Vec<(String, String)>,
    pub href: Option<String>,
}

/// Materialized projection — never written back.
#[derive(Clone, Debug)]
pub struct ViewProjection {
    pub view_id: String,
    pub title: String,
    pub kind: String,
    pub columns: Vec<String>,
    pub rows: Vec<ViewRow>,
    /// group_key → count (kanban columns, inventory classes…)
    pub groups: Vec<(String, usize)>,
    pub summary: String,
}

// ─── storage ──────────────────────────────────────────────────────────

fn ls_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(key).ok().flatten())
}

fn ls_set(key: &str, raw: &str) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(key, raw);
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(key: &str) -> Vec<T> {
    ls_get(key)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(key: &str, list: &[T]) {
    if let Ok(raw) = serde_json::to_string(list) {
        ls_set(key, &raw);
    }
}

pub fn load_cards() -> Vec<Card> {
    load_json(CARDS_KEY)
}
pub fn save_cards(list: &[Card]) {
    save_json(CARDS_KEY, list);
}

pub fn load_erp_intents() -> Vec<ErpIntent> {
    load_json(ERP_INTENTS_KEY)
}
pub fn save_erp_intents(list: &[ErpIntent]) {
    save_json(ERP_INTENTS_KEY, list);
}

pub fn load_templates() -> Vec<UserTemplate> {
    load_json(TEMPLATES_KEY)
}
pub fn save_templates(list: &[UserTemplate]) {
    save_json(TEMPLATES_KEY, list);
}

pub fn load_schedules() -> Vec<Schedule> {
    load_json(SCHEDULES_KEY)
}
pub fn save_schedules(list: &[Schedule]) {
    save_json(SCHEDULES_KEY, list);
}

pub fn load_views() -> Vec<ErpView> {
    load_json(VIEWS_KEY)
}
pub fn save_views(list: &[ErpView]) {
    save_json(VIEWS_KEY, list);
}

fn next_view_id() -> String {
    format!("view-{}", next_intent_id())
}

fn next_intent_id() -> u64 {
    load_erp_intents()
        .iter()
        .map(|i| i.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn slugify(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '-' {
            if !out.ends_with('-') && !out.is_empty() {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    let t = out.trim_matches('-').to_string();
    if t.is_empty() {
        "x".into()
    } else {
        t
    }
}

// ─── seed templates (construct + BOM) ─────────────────────────────────

fn seed_templates() -> Vec<UserTemplate> {
    let mut out = Vec::new();
    // construct seeds (inline — same as before)
    let constructs: &[(&str, &str, &str, &[(&str, f64)], &str)] = &[
        (
            "construct_camp",
            "CONSTRUCT CAMP",
            "Camp kit → camp building",
            &[("camp_kit", 1.0), ("labor", 4.0), ("energy", 2.0)],
            "camp",
        ),
        (
            "construct_cube",
            "CONSTRUCT CUBE",
            "Cube kit → cube building",
            &[
                ("cube_kit", 1.0),
                ("labor", 8.0),
                ("energy", 5.0),
                ("fill", 1.0),
            ],
            "cube",
        ),
        (
            "construct_kitchen",
            "CONSTRUCT KITCHEN",
            "Kitchen node on plot",
            &[
                ("wood", 2.0),
                ("labor", 6.0),
                ("energy", 3.0),
                ("meal", 1.0),
            ],
            "kitchen",
        ),
        (
            "construct_soft",
            "CONSTRUCT SOFT",
            "Soft / event space",
            &[
                ("soft_night", 1.0),
                ("wood", 1.0),
                ("labor", 5.0),
                ("energy", 4.0),
            ],
            "soft",
        ),
        (
            "construct_workshop",
            "CONSTRUCT WORKSHOP",
            "Workshop on plot",
            &[
                ("cube_kit", 1.0),
                ("trail_kit", 1.0),
                ("labor", 10.0),
                ("energy", 6.0),
            ],
            "workshop",
        ),
        (
            "construct_trail",
            "CONSTRUCT TRAIL",
            "Trail segment",
            &[("trail_kit", 1.0), ("labor", 3.0), ("energy", 1.0)],
            "trail",
        ),
        (
            "construct_pad",
            "CONSTRUCT PAD",
            "Hard pad",
            &[
                ("fill", 2.0),
                ("gravel", 1.0),
                ("labor", 4.0),
                ("energy", 2.0),
            ],
            "pad",
        ),
    ];
    for (id, name, blurb, burns, bldg) in constructs {
        out.push(UserTemplate {
            id: (*id).into(),
            name: (*name).into(),
            blurb: (*blurb).into(),
            kind: "construct".into(),
            burns: burns
                .iter()
                .map(|(i, q)| IoLine {
                    id: (*i).into(),
                    qty: *q,
                })
                .collect(),
            mints_coin: vec![],
            mints_building: Some((*bldg).into()),
            needs_plot: true,
            system: true,
        });
    }
    // BOM → transform templates
    for b in BOMS {
        out.push(UserTemplate {
            id: b.id.into(),
            name: b.name.into(),
            blurb: b.blurb.into(),
            kind: "transform".into(),
            burns: b
                .inputs
                .iter()
                .map(|i| IoLine {
                    id: i.id.into(),
                    qty: i.qty,
                })
                .collect(),
            mints_coin: b
                .outputs
                .iter()
                .map(|i| IoLine {
                    id: i.id.into(),
                    qty: i.qty,
                })
                .collect(),
            mints_building: None,
            needs_plot: false,
            system: true,
        });
    }
    out
}

pub fn ensure_templates_seeded() {
    if ls_get(TPL_SEED_KEY).is_some() && !load_templates().is_empty() {
        return;
    }
    // merge seed with any user templates
    let mut user: Vec<UserTemplate> = load_templates().into_iter().filter(|t| !t.system).collect();
    let mut seed = seed_templates();
    seed.append(&mut user);
    save_templates(&seed);
    ls_set(TPL_SEED_KEY, "1");
}

pub fn get_template(id: &str) -> Option<UserTemplate> {
    ensure_templates_seeded();
    load_templates().into_iter().find(|t| t.id == id)
}

// ─── PLUMB ────────────────────────────────────────────────────────────

pub fn plumb_mint_coin(coin_id: &str, qty: f64) {
    if qty > 0.0 {
        stock_add(coin_id, qty);
    }
}

pub fn plumb_mint_card(card: Card) -> Result<Card, String> {
    let mut cards = load_cards();
    if cards.iter().any(|c| c.id == card.id) {
        return Err(format!("card {} exists", card.id));
    }
    cards.push(card.clone());
    save_cards(&cards);
    // every card is a word in the graph — mint its typed particle
    crate::signal::mint_word(
        &card.kind,
        &card.name,
        &card.note,
        &crate::signal::neuron().bech32,
        false,
    );
    Ok(card)
}

pub fn plumb_update_card(id: &str, f: impl FnOnce(&mut Card)) -> Result<(), String> {
    let mut cards = load_cards();
    let c = cards
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("no card {id}"))?;
    if c.locked {
        return Err("card locked".into());
    }
    f(c);
    save_cards(&cards);
    Ok(())
}

pub fn plumb_burn_card(id: &str) -> Result<(), String> {
    let mut cards = load_cards();
    let Some(c) = cards.iter().find(|c| c.id == id) else {
        return Err(format!("no card {id}"));
    };
    if c.locked {
        return Err("cannot burn locked".into());
    }
    cards.retain(|c| c.id != id);
    save_cards(&cards);
    Ok(())
}

// ─── Card CRUD ────────────────────────────────────────────────────────

pub fn create_card(
    kind: &str,
    name: &str,
    class: &str,
    parent: Option<String>,
    zone: &str,
    note: &str,
) -> Result<Card, String> {
    ensure_erp_boot();
    let handle = load_profile().handle;
    let name = name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    let kind = kind.trim().to_lowercase();
    let allowed = [
        "person", "city", "plot", "place", "building", "project", "asset",
    ];
    if !allowed.contains(&kind.as_str()) {
        return Err(format!("kind: {}", allowed.join("|")));
    }
    let id = format!("{}-{}-{}", kind, slugify(name), next_intent_id());
    let card = Card {
        id: id.clone(),
        kind: kind.clone(),
        name: name.into(),
        owner: handle.clone(),
        parent: parent.clone(),
        zone: zone.trim().into(),
        class: if class.trim().is_empty() {
            kind
        } else {
            class.trim().into()
        },
        locked: false,
        note: note.trim().into(),
    };
    plumb_mint_card(card.clone())?;
    // parent relation lands in the graph as a signed signal
    if let Some(ref p) = parent {
        let me = crate::signal::neuron().bech32;
        let child_w = crate::signal::word_particle(&card.kind, &card.name);
        if let Some(pc) = load_cards().into_iter().find(|c| &c.id == p) {
            let parent_w = crate::signal::word_particle(&pc.kind, &pc.name);
            let rel = crate::signal::mint_word("relation", "located_in", "", "", true);
            let _ = crate::signal::emit_signal(
                vec![crate::signal::Link { from: child_w, rel, to: parent_w, weight: 1.0, note: "auto".into() }],
                "card parent",
            );
        }
        let _ = me;
    }
    push_intent(&handle, "mint_card", &id);
    Ok(card)
}

pub fn edit_card(
    id: &str,
    name: &str,
    class: &str,
    zone: &str,
    note: &str,
    parent: Option<String>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name required".into());
    }
    plumb_update_card(id, |c| {
        c.name = name.trim().into();
        c.class = class.trim().into();
        c.zone = zone.trim().into();
        c.note = note.trim().into();
        c.parent = parent;
    })?;
    push_intent(&load_profile().handle, "update_card", id);
    Ok(())
}

pub fn delete_card(id: &str) -> Result<(), String> {
    let kids = children_of(id);
    if !kids.is_empty() {
        return Err(format!("{} children — delete first", kids.len()));
    }
    plumb_burn_card(id)?;
    push_intent(&load_profile().handle, "burn_card", id);
    Ok(())
}

pub fn children_of(parent_id: &str) -> Vec<Card> {
    load_cards()
        .into_iter()
        .filter(|c| c.parent.as_deref() == Some(parent_id))
        .collect()
}

// ─── Coin CRUD ────────────────────────────────────────────────────────

pub fn create_or_add_coin(coin_id: &str, qty: f64) -> Result<String, String> {
    ensure_economy_boot();
    let id = coin_id.trim().to_lowercase().replace(' ', "_");
    if id.is_empty() || qty <= 0.0 {
        return Err("id + qty > 0".into());
    }
    plumb_mint_coin(&id, qty);
    push_intent(&load_profile().handle, "mint_coin", &format!("{id} +{qty}"));
    Ok(format!("+{qty} {id}"))
}

pub fn set_coin_qty(coin_id: &str, qty: f64) -> Result<String, String> {
    ensure_economy_boot();
    let id = coin_id.trim().to_lowercase().replace(' ', "_");
    if id.is_empty() || qty < 0.0 {
        return Err("bad id/qty".into());
    }
    let cur = stock_qty(&id);
    stock_add(&id, qty - cur);
    push_intent(&load_profile().handle, "set_coin", &format!("{id}={qty}"));
    Ok(format!("{id} = {qty}"))
}

// ─── Template CRUD ────────────────────────────────────────────────────

/// Parse "energy:2, labor:1.5" → IoLine list.
pub fn parse_io_blob(s: &str) -> Result<Vec<IoLine>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for part in s.split(&[',', '\n', ';'][..]) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (id, qty_s) = if let Some((a, b)) = part.split_once(':') {
            (a.trim(), b.trim())
        } else if let Some((a, b)) = part.split_once('=') {
            (a.trim(), b.trim())
        } else if let Some((a, b)) = part.split_once(' ') {
            (a.trim(), b.trim())
        } else {
            return Err(format!("bad line '{part}' — use id:qty"));
        };
        let qty: f64 = qty_s.parse().map_err(|_| format!("bad qty in '{part}'"))?;
        if id.is_empty() || qty <= 0.0 {
            return Err(format!("bad io '{part}'"));
        }
        out.push(IoLine {
            id: id.to_lowercase().replace(' ', "_"),
            qty,
        });
    }
    Ok(out)
}

pub fn format_io_blob(lines: &[IoLine]) -> String {
    lines
        .iter()
        .map(|l| format!("{}:{}", l.id, l.qty))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn create_template(
    id: &str,
    name: &str,
    blurb: &str,
    kind: &str,
    burns_blob: &str,
    mints_blob: &str,
    mints_building: &str,
    needs_plot: bool,
) -> Result<UserTemplate, String> {
    ensure_templates_seeded();
    let id = slugify(id);
    if id.is_empty() {
        return Err("template id required".into());
    }
    if load_templates().iter().any(|t| t.id == id) {
        return Err("template id exists".into());
    }
    let name = name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    let burns = parse_io_blob(burns_blob)?;
    let mints_coin = parse_io_blob(mints_blob)?;
    let bldg = {
        let b = mints_building.trim();
        if b.is_empty() {
            None
        } else {
            Some(b.to_lowercase())
        }
    };
    if bldg.is_none() && mints_coin.is_empty() {
        return Err("need mints_coin and/or mints_building".into());
    }
    let t = UserTemplate {
        id: id.clone(),
        name: name.into(),
        blurb: blurb.trim().into(),
        kind: kind.trim().to_lowercase(),
        burns,
        mints_coin,
        mints_building: bldg,
        needs_plot,
        system: false,
    };
    let mut list = load_templates();
    list.insert(0, t.clone());
    save_templates(&list);
    push_intent(&load_profile().handle, "mint_template", &id);
    Ok(t)
}

pub fn update_template(
    id: &str,
    name: &str,
    blurb: &str,
    kind: &str,
    burns_blob: &str,
    mints_blob: &str,
    mints_building: &str,
    needs_plot: bool,
) -> Result<(), String> {
    let burns = parse_io_blob(burns_blob)?;
    let mints_coin = parse_io_blob(mints_blob)?;
    let bldg = {
        let b = mints_building.trim();
        if b.is_empty() {
            None
        } else {
            Some(b.to_lowercase())
        }
    };
    let mut list = load_templates();
    let t = list
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| "template not found".to_string())?;
    t.name = name.trim().into();
    t.blurb = blurb.trim().into();
    t.kind = kind.trim().to_lowercase();
    t.burns = burns;
    t.mints_coin = mints_coin;
    t.mints_building = bldg;
    t.needs_plot = needs_plot;
    save_templates(&list);
    push_intent(&load_profile().handle, "update_template", id);
    Ok(())
}

pub fn delete_template(id: &str) -> Result<(), String> {
    let mut list = load_templates();
    let Some(t) = list.iter().find(|t| t.id == id) else {
        return Err("not found".into());
    };
    if t.system {
        return Err("system template — cannot delete (edit ok)".into());
    }
    // block if schedules reference it
    if load_schedules().iter().any(|s| s.template_id == id) {
        return Err("schedule still uses this template".into());
    }
    list.retain(|t| t.id != id);
    save_templates(&list);
    push_intent(&load_profile().handle, "burn_template", id);
    Ok(())
}

// ─── unified RUN template ─────────────────────────────────────────────

/// Run any template: burns → mints coins and/or building card; intent done.
pub fn run_template(
    template_id: &str,
    plot_flat_id: &str,
    schedule_id: Option<String>,
) -> Result<String, String> {
    ensure_erp_boot();
    ensure_templates_seeded();
    sync_plot_cards_from_leases();

    let t = get_template(template_id).ok_or_else(|| "unknown template".to_string())?;
    let handle = load_profile().handle;

    let plot_card_id = if t.needs_plot {
        if plot_flat_id.is_empty() {
            return Err("select target plot".into());
        }
        // accept plot-xxx or flat id
        let flat = plot_flat_id.strip_prefix("plot-").unwrap_or(plot_flat_id);
        let plot_id = format!("plot-{flat}");
        if !load_cards().iter().any(|c| c.id == plot_id) {
            // try create from lease
            if !load_cards().iter().any(|c| c.id == plot_id) {
                if let Some(l) = load_leases().into_iter().find(|l| l.flat_id == flat) {
                    let _ = plumb_mint_card(Card {
                        id: plot_id.clone(),
                        kind: "plot".into(),
                        name: l.flat_name.clone(),
                        owner: handle.clone(),
                        parent: Some("city-cyber-valley".into()),
                        zone: l.zone.clone(),
                        class: "plot".into(),
                        locked: false,
                        note: format!("lease · {}", l.flat_id),
                    });
                } else {
                    return Err(format!(
                        "no plot card {plot_id} — create CARD kind=plot first"
                    ));
                }
            }
        }
        plot_id
    } else {
        String::new()
    };

    let needs: Vec<(String, f64)> = t.burns.iter().map(|i| (i.id.clone(), i.qty)).collect();
    if !stock_has(&needs) {
        let missing: Vec<String> = needs
            .iter()
            .filter(|(id, q)| stock_qty(id) + 1e-9 < *q)
            .map(|(id, q)| format!("{id} need {q} have {:.1}", stock_qty(id)))
            .collect();
        return Err(format!("missing: {}", missing.join(", ")));
    }

    let iid = next_intent_id();
    let mut intents = load_erp_intents();
    intents.insert(
        0,
        ErpIntent {
            id: iid,
            owner: handle.clone(),
            kind: t.kind.clone(),
            template_id: t.id.clone(),
            state: "reserved".into(),
            target: if plot_card_id.is_empty() {
                None
            } else {
                Some(plot_card_id.clone())
            },
            note: t.name.clone(),
            schedule_id: schedule_id.clone(),
        },
    );
    save_erp_intents(&intents);

    if !stock_consume(&needs) {
        return Err("burn failed".into());
    }

    for m in &t.mints_coin {
        plumb_mint_coin(&m.id, m.qty);
    }

    let mut minted_card = String::new();
    if let Some(ref bclass) = t.mints_building {
        let b_id = format!("bldg-{}-{}-{}", bclass, plot_card_id, iid);
        let lease_name = load_cards()
            .into_iter()
            .find(|c| c.id == plot_card_id)
            .map(|c| c.name)
            .unwrap_or_else(|| plot_card_id.clone());
        let card = Card {
            id: b_id.clone(),
            kind: "building".into(),
            name: format!("{} · {}", bclass.to_uppercase(), lease_name),
            owner: handle.clone(),
            parent: Some(plot_card_id.clone()),
            zone: String::new(),
            class: bclass.clone(),
            locked: false,
            note: format!("template {}", t.id),
        };
        plumb_mint_card(card)?;
        minted_card = b_id;
    }

    let mut intents = load_erp_intents();
    if let Some(it) = intents.iter_mut().find(|i| i.id == iid) {
        it.state = "done".into();
        it.note = if minted_card.is_empty() {
            format!(
                "coins {}",
                t.mints_coin
                    .iter()
                    .map(|m| format!("+{}{}", m.qty, m.id))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            format!("minted {minted_card}")
        };
    }
    save_erp_intents(&intents);

    push_intent(&handle, "run_template", &format!("{} #{}", t.id, iid));

    // the run lands in the graph as one signed signal: agent —ran→ motif;
    // motif —burns→ inputs; motif —mints→ outputs; building —located_in→ plot
    {
        use crate::signal::{emit_signal, mint_word, neuron, Link};
        let me = neuron().bech32;
        let motif = mint_word("concept", &t.id, &t.name, &me, false);
        let person = mint_word("person", &handle, "YOU", &me, false);
        let rel = |name: &str| mint_word("relation", name, "", "", true);
        let mut links = vec![Link {
            from: person,
            rel: rel("ran"),
            to: motif.clone(),
            weight: 1.0,
            note: format!("intent #{iid}"),
        }];
        for b in &t.burns {
            links.push(Link {
                from: motif.clone(),
                rel: rel("burns"),
                to: mint_word("coin", &b.id, "", &me, false),
                weight: b.qty,
                note: format!("-{}", b.qty),
            });
        }
        for m in &t.mints_coin {
            links.push(Link {
                from: motif.clone(),
                rel: rel("mints"),
                to: mint_word("coin", &m.id, "", &me, false),
                weight: m.qty,
                note: format!("+{}", m.qty),
            });
        }
        // cards were minted as words by (kind, name) — resolve the same way
        let card_word = |card_id: &str| -> Option<String> {
            load_cards()
                .into_iter()
                .find(|c| c.id == card_id)
                .map(|c| crate::signal::word_particle(&c.kind, &c.name))
        };
        if let Some(building) = card_word(&minted_card) {
            links.push(Link {
                from: motif.clone(),
                rel: rel("mints"),
                to: building.clone(),
                weight: 1.0,
                note: "building".into(),
            });
            if let Some(plot) = card_word(&plot_card_id) {
                links.push(Link {
                    from: building,
                    rel: rel("located_in"),
                    to: plot,
                    weight: 1.0,
                    note: "construct".into(),
                });
            }
        }
        let _ = emit_signal(links, &format!("motif run · {} · intent #{iid}", t.id));
    }

    Ok(format!("ran {} · intent #{iid} · signal committed", t.name))
}

// ─── Intent CRUD ──────────────────────────────────────────────────────

pub fn create_intent_manual(
    kind: &str,
    template_id: &str,
    target: Option<String>,
    note: &str,
) -> Result<ErpIntent, String> {
    ensure_erp_boot();
    let handle = load_profile().handle;
    let id = next_intent_id();
    let it = ErpIntent {
        id,
        owner: handle.clone(),
        kind: kind.trim().into(),
        template_id: template_id.trim().into(),
        state: "draft".into(),
        target,
        note: note.trim().into(),
        schedule_id: None,
    };
    let mut list = load_erp_intents();
    list.insert(0, it.clone());
    save_erp_intents(&list);
    push_intent(&handle, "intent_draft", &format!("#{id}"));
    Ok(it)
}

pub fn set_intent_state(id: u64, state: &str) -> Result<(), String> {
    let mut list = load_erp_intents();
    let it = list
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or_else(|| "not found".to_string())?;
    it.state = state.into();
    save_erp_intents(&list);
    Ok(())
}

/// Commit draft/reserved intent by running its template.
pub fn commit_intent(id: u64) -> Result<String, String> {
    let it = load_erp_intents()
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "not found".to_string())?;
    if it.state == "done" {
        return Err("already done".into());
    }
    let plot = it
        .target
        .as_deref()
        .unwrap_or("")
        .strip_prefix("plot-")
        .unwrap_or(it.target.as_deref().unwrap_or(""));
    let msg = run_template(&it.template_id, plot, it.schedule_id.clone())?;
    // mark original as cancelled if run created new done intent
    let _ = set_intent_state(id, "done");
    Ok(msg)
}

pub fn delete_intent(id: u64) -> Result<(), String> {
    let mut list = load_erp_intents();
    let n = list.len();
    list.retain(|i| i.id != id);
    if list.len() == n {
        return Err("not found".into());
    }
    save_erp_intents(&list);
    Ok(())
}

// ─── Schedule CRUD ────────────────────────────────────────────────────

pub fn create_schedule(
    name: &str,
    template_id: &str,
    target: Option<String>,
    every_ticks: u32,
    auto_run: bool,
    note: &str,
) -> Result<Schedule, String> {
    ensure_erp_boot();
    ensure_templates_seeded();
    if name.trim().is_empty() {
        return Err("name required".into());
    }
    if get_template(template_id).is_none() {
        return Err("template not found — create template first".into());
    }
    let every = every_ticks.max(1);
    let id = format!("sch-{}-{}", slugify(name), next_intent_id());
    let s = Schedule {
        id: id.clone(),
        name: name.trim().into(),
        template_id: template_id.trim().into(),
        target,
        every_ticks: every,
        tick_count: 0,
        enabled: true,
        auto_run,
        note: note.trim().into(),
    };
    let mut list = load_schedules();
    list.insert(0, s.clone());
    save_schedules(&list);
    push_intent(&load_profile().handle, "mint_schedule", &id);
    Ok(s)
}

pub fn update_schedule(
    id: &str,
    name: &str,
    template_id: &str,
    target: Option<String>,
    every_ticks: u32,
    enabled: bool,
    auto_run: bool,
    note: &str,
) -> Result<(), String> {
    if get_template(template_id).is_none() {
        return Err("template not found".into());
    }
    let mut list = load_schedules();
    let s = list
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| "not found".to_string())?;
    s.name = name.trim().into();
    s.template_id = template_id.trim().into();
    s.target = target;
    s.every_ticks = every_ticks.max(1);
    s.enabled = enabled;
    s.auto_run = auto_run;
    s.note = note.trim().into();
    save_schedules(&list);
    Ok(())
}

pub fn delete_schedule(id: &str) -> Result<(), String> {
    let mut list = load_schedules();
    let n = list.len();
    list.retain(|s| s.id != id);
    if list.len() == n {
        return Err("not found".into());
    }
    save_schedules(&list);
    Ok(())
}

/// Advance all enabled schedules by 1 tick; fire when due.
pub fn tick_schedules() -> Result<String, String> {
    ensure_erp_boot();
    let mut list = load_schedules();
    let mut fired = 0u32;
    let mut msgs = Vec::new();
    for s in list.iter_mut() {
        if !s.enabled {
            continue;
        }
        s.tick_count = s.tick_count.saturating_add(1);
        if s.tick_count >= s.every_ticks {
            s.tick_count = 0;
            fired += 1;
            let plot = s
                .target
                .as_deref()
                .unwrap_or("")
                .strip_prefix("plot-")
                .unwrap_or(s.target.as_deref().unwrap_or(""));
            if s.auto_run {
                match run_template(&s.template_id, plot, Some(s.id.clone())) {
                    Ok(m) => msgs.push(m),
                    Err(e) => {
                        // still spawn draft intent
                        let _ = create_intent_manual(
                            "schedule",
                            &s.template_id,
                            s.target.clone(),
                            &format!("fire failed: {e}"),
                        );
                        msgs.push(format!("{}: {e}", s.name));
                    }
                }
            } else {
                let _ = create_intent_manual(
                    "schedule",
                    &s.template_id,
                    s.target.clone(),
                    &format!("from schedule {}", s.name),
                );
                msgs.push(format!("{} → draft intent", s.name));
            }
        }
    }
    save_schedules(&list);
    Ok(format!("tick · fired {fired} · {}", msgs.join(" · ")))
}

pub fn fire_schedule_now(id: &str) -> Result<String, String> {
    let s = load_schedules()
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "not found".to_string())?;
    let plot = s
        .target
        .as_deref()
        .unwrap_or("")
        .strip_prefix("plot-")
        .unwrap_or(s.target.as_deref().unwrap_or(""));
    if s.auto_run {
        run_template(&s.template_id, plot, Some(s.id))
    } else {
        let it = create_intent_manual(
            "schedule",
            &s.template_id,
            s.target.clone(),
            &format!("manual fire {}", s.name),
        )?;
        Ok(format!("draft intent #{}", it.id))
    }
}

// ─── boot ─────────────────────────────────────────────────────────────

pub fn ensure_erp_boot() {
    ensure_economy_boot();
    ensure_templates_seeded();
    ensure_views_seeded();
    if ls_get(ERP_BOOT_KEY).is_some() {
        sync_plot_cards_from_leases();
        return;
    }
    let handle = load_profile().handle;
    let mut cards = load_cards();
    if !cards.iter().any(|c| c.id == "city-cyber-valley") {
        cards.push(Card {
            id: "city-cyber-valley".into(),
            kind: "city".into(),
            name: "Cyber Valley".into(),
            owner: "cyber-valley".into(),
            parent: None,
            zone: "gesing".into(),
            class: "city".into(),
            locked: false,
            note: "phase-0 · 37 ha".into(),
        });
    }
    let person_id = format!("person-{handle}");
    if !cards.iter().any(|c| c.id == person_id) {
        cards.push(Card {
            id: person_id.clone(),
            kind: "person".into(),
            name: handle.clone(),
            owner: handle.clone(),
            parent: None,
            zone: String::new(),
            class: "agent".into(),
            locked: false,
            note: "YOU".into(),
        });
    }
    save_cards(&cards);
    ls_set(ERP_BOOT_KEY, "1");
    sync_plot_cards_from_leases();
    push_intent("SYSTEM", "erp_boot", "full kernel");
}

pub fn sync_plot_cards_from_leases() {
    use crate::signal::{emit_signal, mint_word, neuron, word_particle, Link};
    let handle = load_profile().handle;
    let mut cards = load_cards();
    let mut changed = false;
    let me = neuron().bech32;
    let mut links: Vec<Link> = Vec::new();
    for l in load_leases() {
        let id = format!("plot-{}", l.flat_id);
        if cards.iter().any(|c| c.id == id) {
            continue;
        }
        cards.push(Card {
            id: id.clone(),
            kind: "plot".into(),
            name: l.flat_name.clone(),
            owner: handle.clone(),
            parent: Some("city-cyber-valley".into()),
            zone: l.zone.clone(),
            class: "plot".into(),
            locked: false,
            note: format!("lease · {}", l.flat_id),
        });
        // the plot enters the graph: word + you —owns→ plot —located_in→ city
        let plot_w = mint_word("plot", &l.flat_name, &format!("lease · {}", l.flat_id), &me, false);
        let you_w = mint_word("person", &handle, "YOU", &me, false);
        let city_w = word_particle("city", "cyber-valley");
        links.push(Link {
            from: you_w,
            rel: mint_word("relation", "owns", "possession — card holds card", "", true),
            to: plot_w.clone(),
            weight: 1.0,
            note: "lease".into(),
        });
        links.push(Link {
            from: plot_w,
            rel: mint_word("relation", "located_in", "spatial containment", "", true),
            to: city_w,
            weight: 1.0,
            note: l.zone.clone(),
        });
        changed = true;
    }
    if changed {
        save_cards(&cards);
        let _ = emit_signal(links, "plot sync — leases into the graph");
    }
}

// ─── VIEWS (derived projections — never mutate) ───────────────────────

pub const VIEW_KINDS: &[&str] = &[
    "lexicon",
    "graph",
    "signals",
    "inventory",
    "balance",
    "kanban",
    "calendar",
    "memory",
    "conservation",
    "cash_flow",
    "flow",
    "custom",
];

fn seed_system_views() -> Vec<ErpView> {
    let handle = load_profile().handle;
    let person = format!("person-{handle}");
    vec![
        ErpView {
            id: "view-inventory".into(),
            name: "Inventory".into(),
            kind: "inventory".into(),
            focus: None,
            filter: String::new(),
            note: "Coin balances grouped by class (TSP-1 ledger)".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-balance".into(),
            name: "Balance · YOU".into(),
            kind: "balance".into(),
            focus: Some(person),
            filter: String::new(),
            note: "Holdings of your person Card".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-kanban".into(),
            name: "Intents kanban".into(),
            kind: "kanban".into(),
            focus: None,
            filter: String::new(),
            note: "Intents grouped by workflow state".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-graph".into(),
            name: "Cybergraph".into(),
            kind: "graph".into(),
            focus: None,
            filter: String::new(),
            note: "Words + links — the committed graph".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-calendar".into(),
            name: "Schedules calendar".into(),
            kind: "calendar".into(),
            focus: None,
            filter: String::new(),
            note: "Which schedules fire which templates".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-memory".into(),
            name: "Memory · ops log".into(),
            kind: "memory".into(),
            focus: None,
            filter: String::new(),
            note: "Operational history (wallet intents + ERP intents)".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-lexicon".into(),
            name: "Lexicon".into(),
            kind: "lexicon".into(),
            focus: None,
            filter: String::new(),
            note: "Words ranked by focus — the living vocabulary".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-signals".into(),
            name: "Signals feed".into(),
            kind: "signals".into(),
            focus: None,
            filter: String::new(),
            note: "Atomic signed batches, committed and draft".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-conservation".into(),
            name: "Conservation".into(),
            kind: "conservation".into(),
            focus: None,
            filter: String::new(),
            note: "Σ held vs minted/burned per coin — honest numbers only".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-cashflow".into(),
            name: "Cash flow".into(),
            kind: "cash_flow".into(),
            focus: None,
            filter: String::new(),
            note: "CX + market orders as soft cash flow".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
        ErpView {
            id: "view-flow".into(),
            name: "Coin flow".into(),
            kind: "flow".into(),
            focus: None,
            filter: String::new(),
            note: "Net coin positions (stocks as flow snapshot)".into(),
            owner: "SYSTEM".into(),
            system: true,
        },
    ]
}

pub fn ensure_views_seeded() {
    if ls_get(VIEW_SEED_KEY).is_some() {
        return;
    }
    let mut list = load_views();
    // drop system views whose kinds no longer exist (pnl/balance-sheet fiction)
    list.retain(|v| !(v.system && !VIEW_KINDS.contains(&v.kind.as_str())));
    for v in seed_system_views() {
        if !list.iter().any(|x| x.id == v.id) {
            list.push(v);
        }
    }
    save_views(&list);
    ls_set(VIEW_SEED_KEY, "1");
}

pub fn get_view(id: &str) -> Option<ErpView> {
    load_views().into_iter().find(|v| v.id == id)
}

pub fn create_view(
    name: &str,
    kind: &str,
    focus: Option<String>,
    filter: &str,
    note: &str,
) -> Result<ErpView, String> {
    ensure_erp_boot();
    let name = name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    let kind = kind.trim().to_ascii_lowercase();
    if kind.is_empty() {
        return Err("kind required".into());
    }
    if !VIEW_KINDS.contains(&kind.as_str()) && kind != "custom" {
        // allow any kind string but prefer known
    }
    let handle = load_profile().handle;
    let v = ErpView {
        id: next_view_id(),
        name: name.into(),
        kind,
        focus: focus.and_then(|f| {
            let t = f.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }),
        filter: filter.trim().into(),
        note: note.trim().into(),
        owner: handle.clone(),
        system: false,
    };
    let mut list = load_views();
    list.insert(0, v.clone());
    save_views(&list);
    // declare the view in the graph: you —declares→ view-word
    {
        use crate::signal::{emit_signal, mint_word, neuron, Link};
        let me = neuron().bech32;
        let you = mint_word("person", &handle, "YOU", &me, false);
        let vw = mint_word("concept", &v.id, &v.name, &me, false);
        let rel = mint_word("relation", "declares", "", "", true);
        let _ = emit_signal(
            vec![Link { from: you, rel, to: vw, weight: 1.0, note: format!("view {}", v.kind) }],
            "view declared",
        );
    }
    push_intent(&handle, "view_create", &v.id);
    Ok(v)
}

pub fn update_view(
    id: &str,
    name: &str,
    kind: &str,
    focus: Option<String>,
    filter: &str,
    note: &str,
) -> Result<(), String> {
    let mut list = load_views();
    let v = list
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| "view not found".to_string())?;
    if v.system {
        // allow edit name/filter/focus/note on system views; keep kind
        v.name = name.trim().into();
        v.focus = focus.and_then(|f| {
            let t = f.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        });
        v.filter = filter.trim().into();
        v.note = note.trim().into();
    } else {
        v.name = name.trim().into();
        if !kind.trim().is_empty() {
            v.kind = kind.trim().to_ascii_lowercase();
        }
        v.focus = focus.and_then(|f| {
            let t = f.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        });
        v.filter = filter.trim().into();
        v.note = note.trim().into();
    }
    save_views(&list);
    Ok(())
}

pub fn delete_view(id: &str) -> Result<(), String> {
    let mut list = load_views();
    let Some(v) = list.iter().find(|v| v.id == id) else {
        return Err("not found".into());
    };
    if v.system {
        return Err("system views cannot be deleted".into());
    }
    list.retain(|v| v.id != id);
    save_views(&list);
    Ok(())
}

fn filt_match(hay: &str, filter: &str) -> bool {
    let f = filter.trim().to_ascii_lowercase();
    if f.is_empty() {
        return true;
    }
    hay.to_ascii_lowercase().contains(&f)
}

/// Materialize a view declaration into a read-only projection.
/// Views never mutate state — only read ledger / graph / intents.
pub fn materialize_view(id: &str) -> Result<ViewProjection, String> {
    ensure_erp_boot();
    let v = get_view(id).ok_or_else(|| "view not found".to_string())?;
    let filter = v.filter.clone();
    let focus = v.focus.clone().unwrap_or_default();

    let (columns, rows, groups, summary) = match v.kind.as_str() {
        "lexicon" => proj_lexicon(&filter),
        "graph" => proj_graph(&focus, &filter),
        "signals" => proj_signals(&filter),
        "inventory" => proj_inventory(&filter),
        "balance" => proj_balance(&focus, &filter),
        "kanban" => proj_kanban(&filter),
        "calendar" => proj_calendar(&filter),
        "memory" => proj_memory(&focus, &filter),
        "conservation" => proj_conservation(&filter),
        "cash_flow" => proj_cash_flow(&filter),
        "flow" => proj_flow(&filter),
        _ => proj_custom(&focus, &filter), // custom = filtered graph links
    };

    Ok(ViewProjection {
        view_id: v.id.clone(),
        title: v.name.clone(),
        kind: v.kind.clone(),
        columns,
        rows,
        groups,
        summary,
    })
}

fn proj_inventory(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::wallet::load_stocks;
    let stocks = load_stocks();
    let mut rows = Vec::new();
    for s in stocks {
        let class = if s.id.contains("kit") || s.id.contains("meal") || s.id.contains("soft") {
            "product"
        } else {
            "element"
        };
        let blob = format!("{} {} {}", s.id, s.qty, class);
        if !filt_match(&blob, filter) {
            continue;
        }
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: class.into(),
            cells: vec![
                ("coin".into(), s.id.clone()),
                ("qty".into(), format!("{:.4}", s.qty)),
                ("class".into(), class.into()),
            ],
            href: Some(format!("/world/coin/{}", s.id)),
        });
    }
    let mut class_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for r in &rows {
        *class_counts.entry(r.tag.clone()).or_insert(0) += 1;
    }
    let groups: Vec<_> = class_counts.into_iter().collect();
    let n = rows.len();
    (
        vec!["coin".into(), "qty".into(), "class".into()],
        rows,
        groups,
        format!("{n} coin lines · inventory (read-only)"),
    )
}

fn proj_balance(
    focus: &str,
    filter: &str,
) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::wallet::{load_balance, load_stocks};
    let bal = load_balance();
    let mut rows = vec![ViewRow {
        key: "cx".into(),
        tag: "numeraire".into(),
        cells: vec![
            ("asset".into(), "CX".into()),
            ("qty".into(), format!("{:.4}", bal.cx)),
            (
                "holder".into(),
                if focus.is_empty() {
                    "YOU".into()
                } else {
                    focus.into()
                },
            ),
        ],
        href: Some("/me".into()),
    }];
    for s in load_stocks() {
        if !filt_match(&format!("{} {}", s.id, s.qty), filter) {
            continue;
        }
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: "coin".into(),
            cells: vec![
                ("asset".into(), s.id.clone()),
                ("qty".into(), format!("{:.4}", s.qty)),
                (
                    "holder".into(),
                    if focus.is_empty() {
                        "YOU".into()
                    } else {
                        focus.into()
                    },
                ),
            ],
            href: Some(format!("/world/coin/{}", s.id)),
        });
    }
    let n = rows.len();
    (
        vec!["asset".into(), "qty".into(), "holder".into()],
        rows,
        vec![("lines".into(), n)],
        format!("balance of {focus} · soft3 local · never mutates"),
    )
}

fn proj_kanban(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    let intents = load_erp_intents();
    let mut rows = Vec::new();
    let mut groups_map: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for it in intents {
        let blob = format!(
            "{} {} {} {} {}",
            it.id, it.state, it.template_id, it.kind, it.note
        );
        if !filt_match(&blob, filter) {
            continue;
        }
        *groups_map.entry(it.state.clone()).or_insert(0) += 1;
        rows.push(ViewRow {
            key: format!("intent-{}", it.id),
            tag: it.state.clone(),
            cells: vec![
                ("id".into(), format!("#{}", it.id)),
                ("state".into(), it.state.clone()),
                ("template".into(), it.template_id.clone()),
                ("kind".into(), it.kind.clone()),
                ("note".into(), it.note.clone()),
            ],
            href: Some(format!("/world/intent/{}", it.id)),
        });
    }
    // stable column order for kanban
    let order = ["draft", "reserved", "done", "cancelled"];
    let mut groups = Vec::new();
    for o in order {
        if let Some(c) = groups_map.remove(o) {
            groups.push((o.into(), c));
        }
    }
    for (k, c) in groups_map {
        groups.push((k, c));
    }
    let n = rows.len();
    (
        vec![
            "id".into(),
            "state".into(),
            "template".into(),
            "kind".into(),
            "note".into(),
        ],
        rows,
        groups,
        format!("{n} intents · kanban by workflow state"),
    )
}

fn proj_graph(
    focus: &str,
    filter: &str,
) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::signal::{draft_links, graph_links, word_name};
    let mut rows = Vec::new();
    let mut by_state: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut all: Vec<(String, crate::signal::Link, &str)> = Vec::new();
    all.extend(graph_links().into_iter().map(|(s, l)| (s, l, "linked")));
    all.extend(draft_links().into_iter().map(|(s, l)| (s, l, "draft")));
    for (i, (sid, l, state)) in all.into_iter().enumerate() {
        if !focus.is_empty() && l.from != focus && l.to != focus && l.rel != focus {
            continue;
        }
        let from_n = word_name(&l.from);
        let rel_n = word_name(&l.rel);
        let to_n = word_name(&l.to);
        let blob = format!("{from_n} {rel_n} {to_n} {state} {}", l.note);
        if !filt_match(&blob, filter) {
            continue;
        }
        *by_state.entry(state.to_string()).or_insert(0) += 1;
        rows.push(ViewRow {
            key: format!("{sid}-{i}"),
            tag: state.to_string(),
            cells: vec![
                ("from".into(), from_n),
                ("rel".into(), rel_n),
                ("to".into(), to_n),
                ("state".into(), state.to_string()),
                ("w".into(), format!("{}", l.weight)),
            ],
            href: Some(format!("/world/signal/{sid}")),
        });
    }
    let n = rows.len();
    let groups: Vec<(String, usize)> = by_state.into_iter().collect();
    (
        vec![
            "from".into(),
            "rel".into(),
            "to".into(),
            "state".into(),
            "w".into(),
        ],
        rows,
        groups,
        format!("{n} links · the graph, committed + draft"),
    )
}

/// The lexicon — every word ranked by focus (Σ committed link weight).
fn proj_lexicon(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    let lex = crate::signal::lexicon();
    let mut rows = Vec::new();
    let mut by_kind: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (w, f) in lex {
        let blob = format!("{} {} {}", w.kind, w.name, w.note);
        if !filt_match(&blob, filter) {
            continue;
        }
        *by_kind.entry(w.kind.clone()).or_insert(0) += 1;
        rows.push(ViewRow {
            key: w.particle.clone(),
            tag: w.kind.clone(),
            cells: vec![
                ("word".into(), w.name.clone()),
                ("kind".into(), w.kind.clone()),
                ("focus".into(), format!("{f:.1}")),
                ("particle".into(), format!("{}…", &w.particle[..8])),
            ],
            href: Some(format!("/world/word/{}", w.particle)),
        });
    }
    let n = rows.len();
    let groups: Vec<(String, usize)> = by_kind.into_iter().collect();
    (
        vec![
            "word".into(),
            "kind".into(),
            "focus".into(),
            "particle".into(),
        ],
        rows,
        groups,
        format!("{n} words · the living vocabulary, ranked by focus"),
    )
}

/// The signal feed — every batch, committed and draft.
fn proj_signals(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    let mut signals = crate::signal::load_signals();
    signals.reverse();
    let mut rows = Vec::new();
    let mut by_state: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for s in signals {
        let blob = format!("{} {} {}", s.id, s.state, s.note);
        if !filt_match(&blob, filter) {
            continue;
        }
        *by_state.entry(s.state.clone()).or_insert(0) += 1;
        let n = s.links.len();
        let run = crate::signal::sentence_run(&s.links);
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: s.state.clone(),
            cells: vec![
                ("signal".into(), s.id.clone()),
                ("state".into(), s.state.clone()),
                ("links".into(), n.to_string()),
                (
                    "shape".into(),
                    if n >= 2 && run == n { "sentence".into() } else { "batch".into() },
                ),
                ("note".into(), s.note.clone()),
            ],
            href: Some(format!("/world/signal/{}", s.id)),
        });
    }
    let n = rows.len();
    let groups: Vec<(String, usize)> = by_state.into_iter().collect();
    (
        vec![
            "signal".into(),
            "state".into(),
            "links".into(),
            "shape".into(),
            "note".into(),
        ],
        rows,
        groups,
        format!("{n} signals · atomic signed batches"),
    )
}

/// Conservation — per coin: current holding + mint/burn links that touch
/// it in the graph. Honest numbers only; no invented coefficients.
fn proj_conservation(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::signal::{graph_links, word_name, word_particle};
    use crate::wallet::load_stocks;
    let links = graph_links();
    let mint_rel = word_particle("relation", "mints");
    let burn_rel = word_particle("relation", "burns");
    let mut rows = Vec::new();
    for s in load_stocks() {
        let coin_w = word_particle("coin", &s.id);
        let minted: f64 = links
            .iter()
            .filter(|(_, l)| l.rel == mint_rel && l.to == coin_w)
            .map(|(_, l)| l.weight)
            .sum();
        let burned: f64 = links
            .iter()
            .filter(|(_, l)| l.rel == burn_rel && l.to == coin_w)
            .map(|(_, l)| l.weight)
            .sum();
        let blob = format!("{} {}", s.id, word_name(&coin_w));
        if !filt_match(&blob, filter) {
            continue;
        }
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: "coin".into(),
            cells: vec![
                ("coin".into(), s.id.clone()),
                ("held".into(), format!("{:.1}", s.qty)),
                ("minted (signals)".into(), format!("{minted:.1}")),
                ("burned (signals)".into(), format!("{burned:.1}")),
            ],
            href: Some(format!("/world/coin/{}", s.id)),
        });
    }
    let n = rows.len();
    (
        vec![
            "coin".into(),
            "held".into(),
            "minted (signals)".into(),
            "burned (signals)".into(),
        ],
        rows,
        vec![],
        format!("{n} coins · Σ held tracks mint − burn as signals accumulate"),
    )
}

fn proj_calendar(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    let mut rows = Vec::new();
    let mut groups: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for s in load_schedules() {
        let blob = format!(
            "{} {} {} {} {}",
            s.id, s.name, s.template_id, s.enabled, s.note
        );
        if !filt_match(&blob, filter) {
            continue;
        }
        let tag = if s.enabled { "enabled" } else { "paused" };
        *groups.entry(tag.into()).or_insert(0) += 1;
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: tag.into(),
            cells: vec![
                ("schedule".into(), s.name.clone()),
                ("template".into(), s.template_id.clone()),
                (
                    "every".into(),
                    format!("{}/{}", s.tick_count, s.every_ticks),
                ),
                ("auto".into(), if s.auto_run { "yes" } else { "no" }.into()),
                ("state".into(), tag.into()),
            ],
            href: Some(format!("/world/schedule/{}", s.id)),
        });
    }
    let n = rows.len();
    (
        vec![
            "schedule".into(),
            "template".into(),
            "every".into(),
            "auto".into(),
            "state".into(),
        ],
        rows,
        groups.into_iter().collect(),
        format!("{n} schedules · soft calendar"),
    )
}

fn proj_memory(
    focus: &str,
    filter: &str,
) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::wallet::load_intents;
    let mut rows = Vec::new();
    // ERP intents
    for it in load_erp_intents() {
        if !focus.is_empty()
            && it.owner != focus
            && it.target.as_deref() != Some(focus)
            && !it.note.contains(focus)
        {
            continue;
        }
        let blob = format!("erp {} {} {} {}", it.id, it.kind, it.template_id, it.note);
        if !filt_match(&blob, filter) {
            continue;
        }
        rows.push(ViewRow {
            key: format!("erp-{}", it.id),
            tag: "erp".into(),
            cells: vec![
                ("src".into(), "erp".into()),
                ("action".into(), it.kind.clone()),
                ("ref".into(), it.template_id.clone()),
                ("state".into(), it.state.clone()),
                ("note".into(), it.note.clone()),
            ],
            href: Some(format!("/world/intent/{}", it.id)),
        });
    }
    // wallet ops log
    for rec in load_intents() {
        let blob = format!("{} {} {}", rec.fleet, rec.action, rec.flat);
        if !focus.is_empty() && !blob.contains(focus) {
            continue;
        }
        if !filt_match(&blob, filter) {
            continue;
        }
        rows.push(ViewRow {
            key: format!("mem-{}", rec.id),
            tag: "ledger".into(),
            cells: vec![
                ("src".into(), "ledger".into()),
                ("action".into(), rec.action.clone()),
                ("ref".into(), rec.flat.clone()),
                ("state".into(), "logged".into()),
                ("note".into(), rec.fleet.clone()),
            ],
            href: None,
        });
    }
    rows.truncate(80);
    let mut g: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for r in &rows {
        *g.entry(r.tag.clone()).or_insert(0) += 1;
    }
    let n = rows.len();
    (
        vec![
            "src".into(),
            "action".into(),
            "ref".into(),
            "state".into(),
            "note".into(),
        ],
        rows,
        g.into_iter().collect(),
        format!("{n} memory rows · history projection"),
    )
}

fn proj_cash_flow(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    // the CX journal — every debit/credit lands here (wallet::LedgerEntry)
    let ledger = crate::wallet::load_ledger();
    let mut rows = Vec::new();
    let mut by_cat: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let (mut tin, mut tout) = (0.0f64, 0.0f64);
    for (i, e) in ledger.iter().enumerate().rev() {
        let blob = format!("{} {} {}", e.cat, e.dir, e.note);
        if !filt_match(&blob, filter) {
            continue;
        }
        if e.dir == "in" {
            tin += e.amount;
        } else {
            tout += e.amount;
        }
        *by_cat.entry(e.cat.clone()).or_insert(0) += 1;
        rows.push(ViewRow {
            key: format!("cf-{i}"),
            tag: e.dir.clone(),
            cells: vec![
                ("cat".into(), e.cat.clone()),
                ("dir".into(), e.dir.clone()),
                (
                    "flow".into(),
                    format!("{}{:.1}", if e.dir == "in" { "+" } else { "-" }, e.amount),
                ),
                ("note".into(), e.note.clone()),
            ],
            href: Some("/me".into()),
        });
    }
    let n = rows.len();
    let groups: Vec<(String, usize)> = by_cat.into_iter().collect();
    (
        vec!["cat".into(), "dir".into(), "flow".into(), "note".into()],
        rows,
        groups,
        format!("{n} movements · in +{tin:.1} · out -{tout:.1} · net {:+.1} CX", tin - tout),
    )
}

fn proj_flow(filter: &str) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    use crate::wallet::load_stocks;
    let mut rows = Vec::new();
    for s in load_stocks() {
        if !filt_match(&s.id, filter) {
            continue;
        }
        let dir = if s.qty > 0.0 { "hold" } else { "empty" };
        rows.push(ViewRow {
            key: s.id.clone(),
            tag: dir.into(),
            cells: vec![
                ("coin".into(), s.id.clone()),
                ("net".into(), format!("{:.4}", s.qty)),
                ("dir".into(), dir.into()),
            ],
            href: Some(format!("/world/coin/{}", s.id)),
        });
    }
    let n = rows.len();
    (
        vec!["coin".into(), "net".into(), "dir".into()],
        rows,
        vec![("positions".into(), n)],
        format!("{n} coin flow positions"),
    )
}

fn proj_custom(
    focus: &str,
    filter: &str,
) -> (Vec<String>, Vec<ViewRow>, Vec<(String, usize)>, String) {
    // custom = free cyberlink filter (+ optional focus)
    proj_graph(focus, filter)
}
