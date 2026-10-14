//! The Ethereum side of a neuron — a wallet connected through the browser's
//! EIP-1193 provider (MetaMask, Rabby, a wallet browser). The address stays
//! in this browser; a plot's token and the DAO live on that chain.

use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub const ETH_KEY: &str = "cyberia.eth.v1";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EthWallet {
    pub address: String,
    /// hex chain id as the provider reports it, `0x1` for mainnet
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub connected_ms: f64,
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

fn provider() -> Option<JsValue> {
    let win = web_sys::window()?;
    let eth = js_sys::Reflect::get(&win, &JsValue::from_str("ethereum")).ok()?;
    (!eth.is_undefined() && !eth.is_null()).then_some(eth)
}

/// A wallet is injected into this browser.
pub fn has_provider() -> bool {
    provider().is_some()
}

async fn request(eth: &JsValue, method: &str) -> Result<JsValue, String> {
    let f: js_sys::Function = js_sys::Reflect::get(eth, &JsValue::from_str("request"))
        .map_err(|_| "the provider has no request()")?
        .dyn_into()
        .map_err(|_| "the provider's request is not a function")?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &JsValue::from_str("method"), &JsValue::from_str(method))
        .map_err(|_| "could not build the request")?;
    let p: js_sys::Promise = f
        .call1(eth, &args)
        .map_err(|e| js_error(&e))?
        .dyn_into()
        .map_err(|_| "the provider returned no promise")?;
    JsFuture::from(p).await.map_err(|e| js_error(&e))
}

fn js_error(e: &JsValue) -> String {
    js_sys::Reflect::get(e, &JsValue::from_str("message"))
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_else(|| "the wallet declined".to_string())
}

/// Ask the wallet for an account; the first one becomes the neuron's address.
pub async fn connect() -> Result<EthWallet, String> {
    let eth = provider().ok_or("no wallet in this browser — install MetaMask or open cyberia.my inside a wallet app")?;
    let accounts = request(&eth, "eth_requestAccounts").await?;
    let address = js_sys::Array::from(&accounts)
        .get(0)
        .as_string()
        .ok_or("the wallet gave no account")?;
    let chain_id = request(&eth, "eth_chainId")
        .await
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let w = EthWallet {
        address,
        chain_id,
        connected_ms: js_sys::Date::now(),
    };
    save_eth(&w);
    Ok(w)
}
