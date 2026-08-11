//! ERP kernel — cyberlink-native.
//! Everything is a cyberlink. Intent = draft cyberlink (first step).
//! Coin · Card · Template · Schedule · PLUMB · View orbit the graph.
//! cyber · cyberia/protocol/system.md · cybergraph

use crate::economy::BOMS;
use crate::wallet::{
    ensure_economy_boot, load_leases, load_profile, push_intent, stock_add, stock_consume,
    stock_has, stock_qty, Lease,
};
use serde::{Deserialize, Serialize};

pub const CARDS_KEY: &str = "cyberia_erp_cards";
pub const ERP_INTENTS_KEY: &str = "cyberia_erp_intents";
pub const TEMPLATES_KEY: &str = "cyberia_erp_templates";
pub const SCHEDULES_KEY: &str = "cyberia_erp_schedules";
pub const BONDS_KEY: &str = "cyberia_erp_bonds";
pub const LINKS_KEY: &str = "cyberia_cyberlinks";
pub const ERP_BOOT_KEY: &str = "cyberia_erp_boot";
pub const TPL_SEED_KEY: &str = "cyberia_erp_tpl_seed_v2";

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

/// Bond — typed link between two Cards (system.md bonds). Prefer Cyberlink.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub id: String,
    pub from: String,
    pub to: String,
    /// owns | works_on | supplies | located_in | uses
    pub rel: String,
    #[serde(default)]
    pub note: String,
}

/// Cyberlink — core cyber primitive: particle → particle.
/// `draft` = intent (first step). `linked` = committed into the graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cyberlink {
    pub id: String,
    /// from particle (card id, coin id, template id, free particle, CID…)
    pub from: String,
    /// to particle
    pub to: String,
    /// predicate / edge type (knows, owns, located_in, burns, mints, intends, …)
    pub rel: String,
    pub owner: String,
    /// draft | linked | cancelled
    pub state: String,
    #[serde(default)]
    pub note: String,
    /// soft focus weight (cyberank stub)
    #[serde(default)]
    pub weight: f64,
    /// optional ops template when link is an action intent
    #[serde(default)]
    pub template_id: Option<String>,
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

pub fn load_bonds() -> Vec<Bond> {
    load_json(BONDS_KEY)
}
pub fn save_bonds(list: &[Bond]) {
    save_json(BONDS_KEY, list);
}

pub fn load_cyberlinks() -> Vec<Cyberlink> {
    load_json(LINKS_KEY)
}
pub fn save_cyberlinks(list: &[Cyberlink]) {
    save_json(LINKS_KEY, list);
}

fn next_link_id() -> String {
    format!("cl-{}", next_intent_id())
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
    // bond: parent relation
    if let Some(ref p) = parent {
        let _ = create_bond(&id, p, "located_in", "auto");
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
    // remove bonds
    let mut bonds = load_bonds();
    bonds.retain(|b| b.from != id && b.to != id);
    save_bonds(&bonds);
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

pub fn template_ready(t: &UserTemplate, plot_flat: &str) -> bool {
    if t.needs_plot && plot_flat.is_empty() {
        return false;
    }
    let needs: Vec<(String, f64)> = t.burns.iter().map(|i| (i.id.clone(), i.qty)).collect();
    stock_has(&needs)
}

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
        let _ = create_bond(&b_id, &plot_card_id, "located_in", "construct");
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

    // cyberlinks: agent —ran→ template; template —mints→ outputs; building —located_in→ plot
    let person = format!("person-{handle}");
    let _ = draft_cyberlink(
        &person,
        &t.id,
        "ran",
        &format!("intent #{iid}"),
        Some(t.id.clone()),
    )
    .and_then(|cl| commit_cyberlink(&cl.id));
    for m in &t.mints_coin {
        let _ = draft_cyberlink(&t.id, &m.id, "mints", &format!("+{}", m.qty), None)
            .and_then(|cl| commit_cyberlink(&cl.id));
    }
    if !minted_card.is_empty() {
        let _ = draft_cyberlink(&t.id, &minted_card, "mints", "building", None)
            .and_then(|cl| commit_cyberlink(&cl.id));
        if !plot_card_id.is_empty() {
            let _ = draft_cyberlink(&minted_card, &plot_card_id, "located_in", "construct", None)
                .and_then(|cl| commit_cyberlink(&cl.id));
        }
    }

    Ok(format!(
        "ran {} · intent #{iid} · cyberlinks written",
        t.name
    ))
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

// ─── CYBERLINK (core of cyber / ERP studio) ───────────────────────────

/// Step 1: INTENT — draft cyberlink (not final in the graph).
pub fn draft_cyberlink(
    from: &str,
    to: &str,
    rel: &str,
    note: &str,
    template_id: Option<String>,
) -> Result<Cyberlink, String> {
    ensure_erp_boot();
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        return Err("from and to particles required".into());
    }
    let handle = load_profile().handle;
    let cl = Cyberlink {
        id: next_link_id(),
        from: from.into(),
        to: to.into(),
        rel: if rel.trim().is_empty() {
            "knows".into()
        } else {
            rel.trim().into()
        },
        owner: handle.clone(),
        state: "draft".into(),
        note: note.trim().into(),
        weight: 1.0,
        template_id,
    };
    let mut list = load_cyberlinks();
    list.insert(0, cl.clone());
    save_cyberlinks(&list);
    let mut intents = load_erp_intents();
    let iid = next_intent_id();
    intents.insert(
        0,
        ErpIntent {
            id: iid,
            owner: handle.clone(),
            kind: "cyberlink".into(),
            template_id: cl.template_id.clone().unwrap_or_else(|| cl.rel.clone()),
            state: "draft".into(),
            target: Some(cl.id.clone()),
            note: format!("{} —[{}]→ {}", cl.from, cl.rel, cl.to),
            schedule_id: None,
        },
    );
    save_erp_intents(&intents);
    push_intent(&handle, "cyberlink_draft", &cl.id);
    Ok(cl)
}

/// Step 2: commit draft → linked cyberlink in the graph.
pub fn commit_cyberlink(id: &str) -> Result<Cyberlink, String> {
    let mut list = load_cyberlinks();
    let cl = list
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| "cyberlink not found".to_string())?;
    if cl.state == "linked" {
        return Ok(cl.clone());
    }
    cl.state = "linked".into();
    cl.weight = (cl.weight + 1.0).min(100.0);
    let out = cl.clone();
    save_cyberlinks(&list);
    let mut intents = load_erp_intents();
    for it in intents.iter_mut() {
        if it.target.as_deref() == Some(id) {
            it.state = "done".into();
        }
    }
    save_erp_intents(&intents);
    push_intent(&load_profile().handle, "cyberlink", id);
    Ok(out)
}

pub fn update_cyberlink(
    id: &str,
    from: &str,
    to: &str,
    rel: &str,
    note: &str,
    weight: f64,
) -> Result<(), String> {
    let mut list = load_cyberlinks();
    let cl = list
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| "not found".to_string())?;
    cl.from = from.trim().into();
    cl.to = to.trim().into();
    cl.rel = rel.trim().into();
    cl.note = note.trim().into();
    cl.weight = weight.max(0.0);
    save_cyberlinks(&list);
    Ok(())
}

pub fn delete_cyberlink(id: &str) -> Result<(), String> {
    let mut list = load_cyberlinks();
    let n = list.len();
    list.retain(|c| c.id != id);
    if list.len() == n {
        return Err("not found".into());
    }
    save_cyberlinks(&list);
    Ok(())
}

pub fn get_cyberlink(id: &str) -> Option<Cyberlink> {
    load_cyberlinks().into_iter().find(|c| c.id == id)
}

pub fn cyberlinks_touching(particle: &str) -> Vec<Cyberlink> {
    load_cyberlinks()
        .into_iter()
        .filter(|c| c.from == particle || c.to == particle)
        .collect()
}

// ─── Bonds (legacy card edges — also write cyberlinks) ────────────────

pub fn create_bond(from: &str, to: &str, rel: &str, note: &str) -> Result<Bond, String> {
    if from.is_empty() || to.is_empty() {
        return Err("from/to required".into());
    }
    let id = format!(
        "bond-{}-{}-{}",
        slugify(rel),
        slugify(from),
        next_intent_id()
    );
    let b = Bond {
        id: id.clone(),
        from: from.into(),
        to: to.into(),
        rel: rel.trim().into(),
        note: note.trim().into(),
    };
    let mut list = load_bonds();
    if list
        .iter()
        .any(|x| x.from == b.from && x.to == b.to && x.rel == b.rel)
    {
        return Ok(b);
    }
    list.push(b.clone());
    save_bonds(&list);
    if let Ok(cl) = draft_cyberlink(&b.from, &b.to, &b.rel, note, None) {
        let _ = commit_cyberlink(&cl.id);
    }
    Ok(b)
}

pub fn delete_bond(id: &str) -> Result<(), String> {
    let mut list = load_bonds();
    let n = list.len();
    list.retain(|b| b.id != id);
    if list.len() == n {
        return Err("not found".into());
    }
    save_bonds(&list);
    Ok(())
}

// ─── boot ─────────────────────────────────────────────────────────────

pub fn ensure_erp_boot() {
    ensure_economy_boot();
    ensure_templates_seeded();
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
    let _ = create_bond(&person_id, "city-cyber-valley", "works_on", "boot");
    ls_set(ERP_BOOT_KEY, "1");
    sync_plot_cards_from_leases();
    push_intent("SYSTEM", "erp_boot", "full kernel");
}

pub fn sync_plot_cards_from_leases() {
    let handle = load_profile().handle;
    let mut cards = load_cards();
    let mut changed = false;
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
        changed = true;
    }
    if changed {
        save_cards(&cards);
    }
}

pub fn plot_value_hint(flat_id: &str) -> f64 {
    let pid = format!("plot-{flat_id}");
    let b = children_of(&pid);
    10.0 + b
        .iter()
        .filter(|c| c.kind == "building")
        .map(|c| match c.class.as_str() {
            "camp" => 8.0,
            "pad" => 6.0,
            "trail" => 5.0,
            "kitchen" => 15.0,
            "soft" => 18.0,
            "workshop" => 25.0,
            "cube" => 40.0,
            _ => 10.0,
        })
        .sum::<f64>()
}

#[derive(Clone, Debug)]
pub struct WorldView {
    pub cards: usize,
    pub coins: usize,
    pub templates: usize,
    pub intents: usize,
    pub schedules: usize,
    pub bonds: usize,
    pub buildings: usize,
    pub cyberlinks: usize,
    pub drafts: usize,
    pub linked: usize,
}

pub fn world_view() -> WorldView {
    use crate::wallet::load_stocks;
    let links = load_cyberlinks();
    WorldView {
        cards: load_cards().len(),
        coins: load_stocks().len(),
        templates: load_templates().len(),
        intents: load_erp_intents().len(),
        schedules: load_schedules().len(),
        bonds: load_bonds().len(),
        buildings: load_cards().iter().filter(|c| c.kind == "building").count(),
        cyberlinks: links.len(),
        drafts: links.iter().filter(|c| c.state == "draft").count(),
        linked: links.iter().filter(|c| c.state == "linked").count(),
    }
}

/// What points at this template.
pub fn template_links(template_id: &str) -> (usize, usize) {
    let intents = load_erp_intents()
        .iter()
        .filter(|i| i.template_id == template_id)
        .count();
    let schedules = load_schedules()
        .iter()
        .filter(|s| s.template_id == template_id)
        .count();
    (intents, schedules)
}

pub fn leases_as_options() -> Vec<Lease> {
    load_leases()
}

// re-export name used by old construct path
pub fn run_construct(template_id: &str, plot_flat_id: &str) -> Result<String, String> {
    run_template(template_id, plot_flat_id, None)
}

pub fn template_can_run(t: &UserTemplate, plot_ok: bool) -> bool {
    if t.needs_plot && !plot_ok {
        return false;
    }
    let needs: Vec<(String, f64)> = t.burns.iter().map(|i| (i.id.clone(), i.qty)).collect();
    stock_has(&needs)
}
