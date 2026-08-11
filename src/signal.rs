//! Signal kernel — the soft3 ladder, local-first, real primitives.
//!
//! word (typed particle) → link (word →rel→ word) → sentence (chain in one
//! signal) → signal (atomic signed batch) → motif (template) → dialect
//! (the cyberia ERP relations) → lexicon (words by focus).
//!
//! Real, not simulated:
//! - a particle is a hemera (Poseidon2/Goldilocks) hash of the word's form
//! - the neuron is a mudra domain-scoped secp256k1 key — the same identity
//!   pipeline the lytics tracker uses (entropy → hemera KDF → d·G)
//! - committing a signal signs its canonical body ADR-036 style; VERIFY
//!   re-checks the signature against the stored pubkey
//!
//! neural/specs: "a signal is the unit of submission: one atomic batch of
//! links". Everything else in the studio projects from this store.

use serde::{Deserialize, Serialize};

pub const SEED_KEY: &str = "cyberia_seed";
pub const WORDS_KEY: &str = "cyberia_words";
pub const SIGNALS_KEY: &str = "cyberia_signals";
pub const SIGNAL_SEQ_KEY: &str = "cyberia_signal_seq";
pub const GRAPH_BOOT_KEY: &str = "cyberia_graph_boot_v1";

pub const DOMAIN: &str = "cyberia.my";
pub const HRP: &str = "cyber";

// ─── storage (same pattern as erp.rs, kept private per module) ────────

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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

// ─── neuron — the signing identity ────────────────────────────────────

/// The local neuron: bech32 wire id, compressed pubkey, hemera native id.
#[derive(Clone, Debug, PartialEq)]
pub struct NeuronInfo {
    pub bech32: String,
    pub pubkey_hex: String,
    pub native_hex: String,
}

fn load_or_create_entropy() -> [u8; 32] {
    if let Some(saved) = ls_get(SEED_KEY).and_then(|s| unhex(&s)) {
        if saved.len() == 32 {
            let mut e = [0u8; 32];
            e.copy_from_slice(&saved);
            return e;
        }
    }
    let mut e = [0u8; 32];
    getrandom::getrandom(&mut e).expect("os entropy");
    ls_set(SEED_KEY, &hex(&e));
    e
}

fn domain_key() -> mudra::domain::DomainKey {
    let entropy = load_or_create_entropy();
    mudra::domain::DomainKey::derive(&entropy, DOMAIN, HRP).expect("domain key derivation")
}

pub fn neuron() -> NeuronInfo {
    let key = domain_key();
    NeuronInfo {
        bech32: key.bech32.clone(),
        pubkey_hex: hex(&key.pubkey),
        native_hex: hex(&key.native),
    }
}

// ─── word — the unit of meaning: a typed particle ─────────────────────

/// Dialect kinds. `relation` words are the predicates links go through;
/// the rest are the cyberia dialect's entity categories (TSP-2-ish) plus
/// `coin` (TSP-1-ish) and free `concept`.
pub const WORD_KINDS: &[&str] = &[
    "concept", "relation", "person", "city", "plot", "place", "building", "project", "asset",
    "coin",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Word {
    /// hemera hash of the form `word:{kind}:{name}` — the global identity.
    pub particle: String,
    /// dialect category (the declared ctype face).
    pub kind: String,
    pub name: String,
    /// annotation — not part of the form; editing it keeps the particle.
    #[serde(default)]
    pub note: String,
    /// bech32 neuron that minted it ("" for system dialect seeds).
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub system: bool,
}

/// The particle of a word form — content addressing, for real.
pub fn word_particle(kind: &str, name: &str) -> String {
    hex(hemera::hash(format!("word:{kind}:{name}").as_bytes()).as_bytes())
}

pub fn load_words() -> Vec<Word> {
    load_json(WORDS_KEY)
}
pub fn save_words(list: &[Word]) {
    save_json(WORDS_KEY, list);
}

pub fn find_word(particle: &str) -> Option<Word> {
    load_words().into_iter().find(|w| w.particle == particle)
}

pub fn find_word_by_name(name: &str) -> Option<Word> {
    let lower = name.to_lowercase();
    load_words()
        .into_iter()
        .find(|w| w.name.to_lowercase() == lower)
}

/// Mint a word if its particle isn't held yet; returns the particle either
/// way. Words are immutable in form — same form, same particle, no dup.
pub fn mint_word(kind: &str, name: &str, note: &str, owner: &str, system: bool) -> String {
    let particle = word_particle(kind, name);
    let mut words = load_words();
    if !words.iter().any(|w| w.particle == particle) {
        words.push(Word {
            particle: particle.clone(),
            kind: kind.to_string(),
            name: name.to_string(),
            note: note.to_string(),
            owner: owner.to_string(),
            system,
        });
        save_words(&words);
    }
    particle
}

/// Resolve free text into a word particle: an existing name matches
/// case-insensitively; anything new mints a `concept` word on the fly.
/// A 64-char hex string is accepted as a raw particle reference.
pub fn resolve_word(text: &str, owner: &str) -> String {
    let t = text.trim();
    if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        return t.to_lowercase();
    }
    if let Some(w) = find_word_by_name(t) {
        return w.particle;
    }
    mint_word("concept", t, "", owner, false)
}

// ─── link + signal — the atom and its unit of submission ──────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// from-word particle
    pub from: String,
    /// relation-word particle — the predicate is itself a word
    pub rel: String,
    /// to-word particle
    pub to: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default)]
    pub note: String,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub id: String,
    /// bech32 neuron
    pub neuron: String,
    /// draft | committed
    pub state: String,
    pub links: Vec<Link>,
    /// hemera hash of the canonical body (hex) — set on commit
    #[serde(default)]
    pub body_particle: String,
    /// compressed pubkey hex — set on commit
    #[serde(default)]
    pub pubkey_hex: String,
    /// ADR-036 signature hex — set on commit
    #[serde(default)]
    pub sig_hex: String,
    #[serde(default)]
    pub note: String,
}

pub fn load_signals() -> Vec<Signal> {
    load_json(SIGNALS_KEY)
}
pub fn save_signals(list: &[Signal]) {
    save_json(SIGNALS_KEY, list);
}

/// Monotonic id — its own counter, immune to deletions elsewhere.
fn next_signal_id() -> String {
    let n: u64 = ls_get(SIGNAL_SEQ_KEY)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
        + 1;
    ls_set(SIGNAL_SEQ_KEY, &n.to_string());
    format!("sg-{n}")
}

pub fn find_signal(id: &str) -> Option<Signal> {
    load_signals().into_iter().find(|s| s.id == id)
}

/// The one open draft signal — the batch links accumulate into. Created
/// on demand; committing it opens the next.
pub fn open_draft() -> Signal {
    let mut signals = load_signals();
    if let Some(s) = signals.iter().find(|s| s.state == "draft") {
        return s.clone();
    }
    let s = Signal {
        id: next_signal_id(),
        neuron: neuron().bech32,
        state: "draft".into(),
        links: Vec::new(),
        body_particle: String::new(),
        pubkey_hex: String::new(),
        sig_hex: String::new(),
        note: String::new(),
    };
    signals.push(s.clone());
    save_signals(&signals);
    s
}

/// Add a link to the open draft. Returns the draft signal id.
pub fn draft_link(from: &str, rel: &str, to: &str, weight: f64, note: &str) -> String {
    let draft = open_draft();
    let mut signals = load_signals();
    if let Some(s) = signals.iter_mut().find(|s| s.id == draft.id) {
        s.links.push(Link {
            from: from.to_string(),
            rel: rel.to_string(),
            to: to.to_string(),
            weight,
            note: note.to_string(),
        });
    }
    save_signals(&signals);
    draft.id
}

pub fn remove_draft_link(signal_id: &str, index: usize) {
    let mut signals = load_signals();
    if let Some(s) = signals
        .iter_mut()
        .find(|s| s.id == signal_id && s.state == "draft")
    {
        if index < s.links.len() {
            s.links.remove(index);
        }
    }
    save_signals(&signals);
}

/// Canonical body — deterministic line encoding, one link per line.
/// This is what gets hashed and signed; stable across serde details.
pub fn canonical_body(links: &[Link]) -> String {
    links
        .iter()
        .map(|l| format!("{}|{}|{}|{:.6}", l.from, l.rel, l.to, l.weight))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Commit = hash the canonical body (hemera) + sign it (ADR-036) with the
/// domain neuron. An empty draft refuses to commit.
pub fn commit_signal(id: &str) -> Result<(), String> {
    let mut signals = load_signals();
    let Some(s) = signals.iter_mut().find(|s| s.id == id) else {
        return Err("signal not found".into());
    };
    if s.state != "draft" {
        return Err("already committed".into());
    }
    if s.links.is_empty() {
        return Err("empty signal — add at least one link".into());
    }
    let key = domain_key();
    let body = canonical_body(&s.links);
    let body_bytes = body.as_bytes();
    s.body_particle = hex(hemera::hash(body_bytes).as_bytes());
    s.neuron = key.bech32.clone();
    s.pubkey_hex = hex(&key.pubkey);
    s.sig_hex = hex(&mudra::claim::sign_arbitrary(
        key.signing_key(),
        &key.bech32,
        body_bytes,
    ));
    s.state = "committed".into();
    save_signals(&signals);
    Ok(())
}

/// Re-verify a committed signal: pubkey → bech32 must match the neuron
/// field, body must re-hash to body_particle, signature must verify.
pub fn verify_signal(id: &str) -> Result<(), String> {
    let Some(s) = find_signal(id) else {
        return Err("signal not found".into());
    };
    if s.state != "committed" {
        return Err("not committed".into());
    }
    let body = canonical_body(&s.links);
    if hex(hemera::hash(body.as_bytes()).as_bytes()) != s.body_particle {
        return Err("body particle mismatch".into());
    }
    let pubkey: [u8; 33] = unhex(&s.pubkey_hex)
        .and_then(|v| v.try_into().ok())
        .ok_or("bad pubkey encoding")?;
    match mudra::cosmos::address(&pubkey, HRP) {
        Ok(addr) if addr == s.neuron => {}
        Ok(_) => return Err("neuron does not match pubkey".into()),
        Err(e) => return Err(format!("address: {e}")),
    }
    let sig: [u8; 64] = unhex(&s.sig_hex)
        .and_then(|v| v.try_into().ok())
        .ok_or("bad signature encoding")?;
    if mudra::claim::verify_arbitrary(&pubkey, &s.neuron, body.as_bytes(), &sig) {
        Ok(())
    } else {
        Err("signature INVALID".into())
    }
}

/// Emit a committed, signed signal in one step — the ops path (template
/// runs, syncs). Refuses empty batches.
pub fn emit_signal(links: Vec<Link>, note: &str) -> Result<String, String> {
    if links.is_empty() {
        return Err("empty signal".into());
    }
    let mut signals = load_signals();
    let id = next_signal_id();
    signals.push(Signal {
        id: id.clone(),
        neuron: neuron().bech32,
        state: "draft".into(),
        links,
        body_particle: String::new(),
        pubkey_hex: String::new(),
        sig_hex: String::new(),
        note: note.to_string(),
    });
    save_signals(&signals);
    commit_signal(&id)?;
    Ok(id)
}

// ─── graph reads ──────────────────────────────────────────────────────

/// All committed links — the graph.
pub fn graph_links() -> Vec<(String, Link)> {
    load_signals()
        .into_iter()
        .filter(|s| s.state == "committed")
        .flat_map(|s| {
            let id = s.id.clone();
            s.links.into_iter().map(move |l| (id.clone(), l))
        })
        .collect()
}

/// Draft links (the open batch), for kanban/pending views.
pub fn draft_links() -> Vec<(String, Link)> {
    load_signals()
        .into_iter()
        .filter(|s| s.state == "draft")
        .flat_map(|s| {
            let id = s.id.clone();
            s.links.into_iter().map(move |l| (id.clone(), l))
        })
        .collect()
}

/// Links touching a word (as from, to, or relation), committed only.
pub fn links_touching(particle: &str) -> Vec<(String, Link)> {
    graph_links()
        .into_iter()
        .filter(|(_, l)| l.from == particle || l.to == particle || l.rel == particle)
        .collect()
}

/// Soft focus — Σ weight of committed links touching the word. A local
/// φ* stub: honest about being degree-weight, not the tri-kernel.
pub fn word_focus(particle: &str) -> f64 {
    graph_links()
        .iter()
        .map(|(_, l)| {
            let mut f = 0.0;
            if l.from == particle {
                f += l.weight;
            }
            if l.to == particle {
                f += l.weight;
            }
            if l.rel == particle {
                f += l.weight;
            }
            f
        })
        .sum()
}

/// The lexicon — words ranked by focus, then name. The living vocabulary.
pub fn lexicon() -> Vec<(Word, f64)> {
    let mut out: Vec<(Word, f64)> = load_words()
        .into_iter()
        .map(|w| {
            let f = word_focus(&w.particle);
            (w, f)
        })
        .collect();
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.name.cmp(&b.0.name))
    });
    out
}

/// Sentence detection: the longest chain from the start where each link's
/// `to` feeds the next link's `from`. Full-length chain = a sentence.
pub fn sentence_run(links: &[Link]) -> usize {
    if links.len() < 2 {
        return links.len();
    }
    let mut run = 1;
    for pair in links.windows(2) {
        if pair[0].to == pair[1].from {
            run += 1;
        } else {
            break;
        }
    }
    run
}

/// Display name for a particle — the word's name, or a short hex stub.
pub fn word_name(particle: &str) -> String {
    find_word(particle)
        .map(|w| w.name)
        .unwrap_or_else(|| format!("{}…", &particle[..particle.len().min(8)]))
}

// ─── dialect seed + migration from the old ERP keys ───────────────────

/// The cyberia dialect's relation vocabulary. PLUMB verbs and the bond
/// relations become relation-words — the predicates of the graph.
pub const DIALECT_RELATIONS: &[(&str, &str)] = &[
    ("owns", "possession — card holds card"),
    ("located_in", "spatial containment"),
    ("works_on", "labor relation"),
    ("supplies", "provision relation"),
    ("uses", "usage relation"),
    ("knows", "acquaintance / reference"),
    ("burns", "PLUMB burn — consumes coin"),
    ("mints", "PLUMB mint — creates token"),
    ("pays", "PLUMB pay — transfers coin"),
    ("locks", "PLUMB lock — constrains"),
    ("updates", "PLUMB update — reconfigures"),
    ("intends", "draft assertion — proof in progress"),
    ("priced_in", "denomination relation"),
    ("member_of", "membership relation"),
];

/// Old-ERP shapes we migrate from (minimal mirrors of erp.rs structs).
#[derive(Deserialize)]
struct OldCard {
    id: String,
    kind: String,
    name: String,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct OldLink {
    from: String,
    to: String,
    rel: String,
    state: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    weight: f64,
}

#[derive(Deserialize)]
struct OldBond {
    from: String,
    to: String,
    rel: String,
}

/// Boot: seed the dialect, migrate old cards/cyberlinks/bonds into
/// words + one genesis signal. Runs once (flag key).
pub fn ensure_graph_boot() {
    if ls_get(GRAPH_BOOT_KEY).is_some() {
        return;
    }
    let me = neuron();

    // dialect relations
    for (name, note) in DIALECT_RELATIONS {
        mint_word("relation", name, note, "", true);
    }

    // cards → words (id → particle map for link migration); the boot
    // person/city cards from ensure_erp_boot arrive through this same path,
    // so there is exactly ONE word per entity — no parallel specials
    let mut idmap: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in load_json::<OldCard>("cyberia_erp_cards") {
        let p = mint_word(&c.kind, &c.name, &c.note, &me.bech32, false);
        idmap.insert(c.id, p);
    }

    let mut resolve = |s: &str, idmap: &std::collections::HashMap<String, String>| -> String {
        if let Some(p) = idmap.get(s) {
            return p.clone();
        }
        resolve_word(s, &me.bech32)
    };

    // old committed cyberlinks + bonds → one genesis signal
    let mut genesis: Vec<Link> = Vec::new();
    let mut drafts: Vec<Link> = Vec::new();
    for l in load_json::<OldLink>("cyberia_cyberlinks") {
        let link = Link {
            from: resolve(&l.from, &idmap),
            rel: mint_word("relation", &l.rel, "", "", false),
            to: resolve(&l.to, &idmap),
            weight: if l.weight > 0.0 { l.weight } else { 1.0 },
            note: l.note,
        };
        if l.state == "linked" {
            genesis.push(link);
        } else if l.state == "draft" {
            drafts.push(link);
        }
    }
    for b in load_json::<OldBond>("cyberia_erp_bonds") {
        genesis.push(Link {
            from: resolve(&b.from, &idmap),
            rel: mint_word("relation", &b.rel, "", "", false),
            to: resolve(&b.to, &idmap),
            weight: 1.0,
            note: "migrated bond".into(),
        });
    }
    if !genesis.is_empty() {
        let _ = emit_signal(genesis, "genesis — migrated from the old ERP graph");
    } else {
        // fresh universe: the first committed signal binds the person to the
        // city — both words came in through the card migration above
        let words = load_words();
        let you = words.iter().find(|w| w.kind == "person").cloned();
        let city = words.iter().find(|w| w.kind == "city").cloned();
        if let (Some(you), Some(city)) = (you, city) {
            let works_on = mint_word("relation", "works_on", "labor relation", "", true);
            let _ = emit_signal(
                vec![Link {
                    from: you.particle,
                    rel: works_on,
                    to: city.particle,
                    weight: 1.0,
                    note: "boot".into(),
                }],
                "boot — you work on the first city",
            );
        }
    }
    if !drafts.is_empty() {
        let d = open_draft();
        let mut signals = load_signals();
        if let Some(s) = signals.iter_mut().find(|s| s.id == d.id) {
            s.links.extend(drafts);
            s.note = "migrated drafts".into();
        }
        save_signals(&signals);
    }

    // the old Bond store is fully subsumed by links — drop it
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.remove_item("cyberia_erp_bonds");
    }

    ls_set(GRAPH_BOOT_KEY, "1");
}

/// Counts for the hub.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphView {
    pub words: usize,
    pub relations: usize,
    pub links: usize,
    pub drafts: usize,
    pub signals: usize,
    pub committed: usize,
}

pub fn graph_view() -> GraphView {
    let words = load_words();
    let signals = load_signals();
    let relations = words.iter().filter(|w| w.kind == "relation").count();
    let links = signals
        .iter()
        .filter(|s| s.state == "committed")
        .map(|s| s.links.len())
        .sum();
    let drafts = signals
        .iter()
        .filter(|s| s.state == "draft")
        .map(|s| s.links.len())
        .sum();
    let committed = signals.iter().filter(|s| s.state == "committed").count();
    GraphView {
        words: words.len(),
        relations,
        links,
        drafts,
        signals: signals.len(),
        committed,
    }
}
