//! The Ethereum side of a neuron — a wallet connected through the browser.
//!
//! A browser can carry several wallets at once (MetaMask, Rabby, Brave's
//! own, a phone wallet's in-app browser), and the old single `window.ethereum`
//! door belongs to whichever of them grabbed it — often one the person never
//! set up. So the wallets are discovered the way they announce themselves
//! (EIP-6963), the person picks one by name, and the choice is remembered
//! with the address. `window.ethereum` stays as the fallback for wallets
//! that do not announce. The address stays in this browser; a plot's token
//! and the DAO live on that chain.

use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub const ETH_KEY: &str = "cyberia.eth.v1";
/// The key of the wallet that sits behind plain `window.ethereum`.
const INJECTED: &str = "injected";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EthWallet {
    pub address: String,
    /// hex chain id as the provider reports it, `0x1` for mainnet
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub connected_ms: f64,
    /// which wallet it is — its reverse-dns name (`io.metamask`), or `injected`
    #[serde(default)]
    pub rdns: String,
    /// the wallet's own name, as it announced it
    #[serde(default)]
    pub wallet: String,
}

impl EthWallet {
    /// `0x1234…abcd`
    pub fn short(&self) -> String {
        let a = &self.address;
        if a.len() > 12 {
            format!("{}…{}", &a[..6], &a[a.len() - 4..])
        } else {
            a.clone()
        }
    }

    pub fn chain_name(&self) -> &'static str {
        match self.chain_id.as_str() {
            "0x1" => "Ethereum",
            "0x2105" => "Base",
            "0xaa36a7" => "Sepolia",
            "0x89" => "Polygon",
            "0xa4b1" => "Arbitrum",
            "0xa" => "Optimism",
            _ => "chain",
        }
    }
}

/// A wallet found in this browser.
#[derive(Clone, Debug, PartialEq)]
pub struct WalletInfo {
    /// what `connect_with` takes: the wallet's rdns, or `injected`
    pub key: String,
    pub name: String,
    /// the wallet's icon as a data address, when it gave one
    pub icon: String,
}

thread_local! {
    static FOUND: RefCell<Vec<(WalletInfo, JsValue)>> = const { RefCell::new(Vec::new()) };
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

pub fn load_eth() -> Option<EthWallet> {
    storage()
        .and_then(|ls| ls.get_item(ETH_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(|w: &EthWallet| !w.address.is_empty())
}

pub fn save_eth(w: &EthWallet) {
    if let (Some(ls), Ok(s)) = (storage(), serde_json::to_string(w)) {
        let _ = ls.set_item(ETH_KEY, &s);
    }
}

pub fn forget_eth() {
    if let Some(ls) = storage() {
        let _ = ls.remove_item(ETH_KEY);
    }
}

fn get(obj: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(obj, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn injected() -> Option<JsValue> {
    let win = web_sys::window()?;
    let eth = get(&win, "ethereum");
    (!eth.is_undefined() && !eth.is_null()).then_some(eth)
}

/// Ask the wallets of this browser to announce themselves, once. Their
/// answers arrive as events and are kept; call `wallets()` after.
pub fn discover() {
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    let Some(win) = web_sys::window() else { return };
    let on_announce = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let detail = get(&ev, "detail");
        let (info, provider) = (get(&detail, "info"), get(&detail, "provider"));
        if provider.is_undefined() || provider.is_null() {
            return;
        }
        let text = |k: &str| get(&info, k).as_string().unwrap_or_default();
        let rdns = text("rdns");
        let key = if rdns.is_empty() { text("uuid") } else { rdns };
        if key.is_empty() {
            return;
        }
        let icon = text("icon");
        let w = WalletInfo {
            key: key.clone(),
            name: { let n = text("name"); if n.is_empty() { "a wallet".to_string() } else { n } },
            // only an inline picture: a wallet's icon must not make the page fetch from elsewhere
            icon: if icon.starts_with("data:image/") { icon } else { String::new() },
        };
        FOUND.with(|f| {
            let mut f = f.borrow_mut();
            if !f.iter().any(|(i, _)| i.key == key) {
                f.push((w, provider));
            }
        });
    });
    let _ = win.add_event_listener_with_callback("eip6963:announceProvider", on_announce.as_ref().unchecked_ref());
    on_announce.forget();
    if let Ok(ev) = web_sys::Event::new("eip6963:requestProvider") {
        let _ = win.dispatch_event(&ev);
    }
}

/// The wallets of this browser: those that announced themselves, or the one
/// behind `window.ethereum` when none did.
pub fn wallets() -> Vec<WalletInfo> {
    let found: Vec<WalletInfo> = FOUND.with(|f| f.borrow().iter().map(|(i, _)| i.clone()).collect());
    if !found.is_empty() {
        return found;
    }
    match injected() {
        Some(eth) => {
            let is = |k: &str| get(&eth, k).as_bool().unwrap_or(false);
            let name = if is("isRabby") {
                "Rabby"
            } else if is("isBraveWallet") {
                "Brave Wallet"
            } else if is("isCoinbaseWallet") {
                "Coinbase Wallet"
            } else if is("isMetaMask") {
                "MetaMask"
            } else {
                "the browser's wallet"
            };
            vec![WalletInfo { key: INJECTED.to_string(), name: name.to_string(), icon: String::new() }]
        }
        None => Vec::new(),
    }
}

/// The provider of a wallet by its key; with no key, the one the person
/// connected before, then the only one there is, then `window.ethereum`.
fn provider(key: Option<&str>) -> Option<(JsValue, WalletInfo)> {
    let by = |k: &str| FOUND.with(|f| f.borrow().iter().find(|(i, _)| i.key == k).map(|(i, p)| (p.clone(), i.clone())));
    let fallback = || {
        injected().map(|p| {
            let info = wallets().into_iter().find(|w| w.key == INJECTED).unwrap_or(WalletInfo {
                key: INJECTED.to_string(),
                name: "the browser's wallet".to_string(),
                icon: String::new(),
            });
            (p, info)
        })
    };
    match key {
        Some(INJECTED) => fallback(),
        Some(k) => by(k),
        None => load_eth()
            .filter(|w| !w.rdns.is_empty())
            .and_then(|w| if w.rdns == INJECTED { fallback() } else { by(&w.rdns) })
            .or_else(|| {
                let n = FOUND.with(|f| f.borrow().len());
                if n == 1 { FOUND.with(|f| f.borrow().first().map(|(i, p)| (p.clone(), i.clone()))) } else { None }
            })
            .or_else(fallback),
    }
}

async fn request(eth: &JsValue, method: &str, params: Option<js_sys::Array>) -> Result<JsValue, String> {
    let f: js_sys::Function = get(eth, "request").dyn_into().map_err(|_| "this wallet cannot be asked — update it or pick another")?;
    let args = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&args, &JsValue::from_str("method"), &JsValue::from_str(method));
    if let Some(p) = params {
        let _ = js_sys::Reflect::set(&args, &JsValue::from_str("params"), &p);
    }
    let p: js_sys::Promise = f
        .call1(eth, &args)
        .map_err(|e| human(&e))?
        .dyn_into()
        .map_err(|_| "the wallet did not answer — update it or pick another")?;
    JsFuture::from(p).await.map_err(|e| human(&e))
}

/// A wallet's refusal in words a person can act on.
fn human(e: &JsValue) -> String {
    let code = get(e, "code").as_f64().unwrap_or(0.0) as i64;
    let msg = get(e, "message").as_string().unwrap_or_default();
    let low = msg.to_lowercase();
    if code == 4001 || low.contains("user rejected") || low.contains("user denied") {
        "you declined it in the wallet — press again when ready".to_string()
    } else if code == -32002 || low.contains("already pending") {
        "the wallet already has a request open — open the wallet's window and answer it".to_string()
    } else if low.contains("at least one account") || low.contains("no account") {
        "this wallet has no account yet — open it and create or import one, or pick another wallet".to_string()
    } else if low.contains("lock") {
        "the wallet is locked — unlock it and press again".to_string()
    } else if msg.is_empty() {
        "the wallet declined".to_string()
    } else {
        format!("the wallet says: {msg}")
    }
}

/// Ask a wallet for an account. `key` names the wallet (`wallets()`); with
/// none, the one connected before or the only one there is.
pub async fn connect_with(key: Option<String>) -> Result<EthWallet, String> {
    discover();
    let (eth, info) = provider(key.as_deref())
        .ok_or("no wallet in this browser — install MetaMask or Rabby, or open cyberia.my inside a wallet app")?;
    let accounts = request(&eth, "eth_requestAccounts", None).await?;
    let address = js_sys::Array::from(&accounts)
        .get(0)
        .as_string()
        .ok_or("the wallet gave no account — open it, create or unlock an account, and press again")?;
    let chain_id = request(&eth, "eth_chainId", None).await.ok().and_then(|v| v.as_string()).unwrap_or_default();
    let w = EthWallet {
        address,
        chain_id,
        connected_ms: js_sys::Date::now(),
        rdns: info.key,
        wallet: info.name,
    };
    save_eth(&w);
    Ok(w)
}


/// `personal_sign` (EIP-191): the wallet shows `message` as text and signs
/// it with `address`. Asked of the wallet that address was connected from.
pub async fn personal_sign(address: &str, message: &str) -> Result<String, String> {
    discover();
    let (eth, _) = provider(None).ok_or("the wallet is gone from this browser — connect it again")?;
    let params = js_sys::Array::new();
    params.push(&JsValue::from_str(message));
    params.push(&JsValue::from_str(address));
    request(&eth, "personal_sign", Some(params))
        .await?
        .as_string()
        .ok_or_else(|| "the wallet returned no signature".to_string())
}
