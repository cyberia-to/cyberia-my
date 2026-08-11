//! Elements, products, BOM chains, market fill — soft3 local production loop.
//! No organizations: only YOU stocks + city seed book + peer orders.

use crate::wallet::{
    credit_cx, debit_cx, ensure_economy_boot, load_balance, load_orders, load_profile,
    next_order_id, push_intent, save_orders, stock_add, stock_consume, stock_has, stock_qty,
    MarketOrder,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoodKind {
    Element,
    Product,
}

#[derive(Clone, Copy, Debug)]
pub struct GoodDef {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub kind: GoodKind,
    pub class: &'static str, // energy | matter | labor | digital | food | kit | stay
    pub blurb: &'static str,
}

/// Full element + product catalog (v1).
pub const GOODS: &[GoodDef] = &[
    // —— elements (primitives) ——
    GoodDef {
        id: "energy",
        name: "ENERGY",
        unit: "kWh",
        kind: GoodKind::Element,
        class: "energy",
        blurb: "Electric work — solar, genset, grid soft.",
    },
    GoodDef {
        id: "water",
        name: "WATER",
        unit: "m³",
        kind: GoodKind::Element,
        class: "matter",
        blurb: "Stored / pumped water for grow and camp.",
    },
    GoodDef {
        id: "labor",
        name: "LABOR",
        unit: "h",
        kind: GoodKind::Element,
        class: "labor",
        blurb: "Person-hours — you or robots as soft stock.",
    },
    GoodDef {
        id: "wood",
        name: "WOOD",
        unit: "m³",
        kind: GoodKind::Element,
        class: "matter",
        blurb: "Timber / biomass feedstock.",
    },
    GoodDef {
        id: "food_raw",
        name: "FOOD RAW",
        unit: "kg",
        kind: GoodKind::Element,
        class: "food",
        blurb: "Unprocessed garden / market produce.",
    },
    GoodDef {
        id: "fill",
        name: "FILL",
        unit: "m³",
        kind: GoodKind::Element,
        class: "matter",
        blurb: "Earth fill for terraces and pads.",
    },
    GoodDef {
        id: "gravel",
        name: "GRAVEL",
        unit: "t",
        kind: GoodKind::Element,
        class: "matter",
        blurb: "Aggregate for trails and bases.",
    },
    GoodDef {
        id: "biochar",
        name: "BIOCHAR",
        unit: "kg",
        kind: GoodKind::Element,
        class: "matter",
        blurb: "Carbon fixed in soil — burn.city metabolism.",
    },
    GoodDef {
        id: "bandwidth",
        name: "BANDWIDTH",
        unit: "GB",
        kind: GoodKind::Element,
        class: "digital",
        blurb: "Uplink / mesh transfer budget.",
    },
    GoodDef {
        id: "compute",
        name: "COMPUTE",
        unit: "GPU·h",
        kind: GoodKind::Element,
        class: "digital",
        blurb: "Local or rented inference / train hours.",
    },
    // —— products (BOM outputs) ——
    GoodDef {
        id: "meal",
        name: "MEAL",
        unit: "ea",
        kind: GoodKind::Product,
        class: "food",
        blurb: "Cooked meal — kitchen product.",
    },
    GoodDef {
        id: "camp_kit",
        name: "CAMP KIT",
        unit: "kit",
        kind: GoodKind::Product,
        class: "kit",
        blurb: "Overnight camp build kit.",
    },
    GoodDef {
        id: "cube_kit",
        name: "CUBE KIT",
        unit: "kit",
        kind: GoodKind::Product,
        class: "kit",
        blurb: "Cube / hard-shell build stack.",
    },
    GoodDef {
        id: "trail_kit",
        name: "TRAIL KIT",
        unit: "kit",
        kind: GoodKind::Product,
        class: "kit",
        blurb: "Trail repair package.",
    },
    GoodDef {
        id: "biochar_bag",
        name: "BIOCHAR BAG",
        unit: "bag",
        kind: GoodKind::Product,
        class: "kit",
        blurb: "Bagged biochar for soil.",
    },
    GoodDef {
        id: "soft_night",
        name: "SOFT NIGHT",
        unit: "night",
        kind: GoodKind::Product,
        class: "stay",
        blurb: "Hospitality night — meal + energy + care.",
    },
];

pub fn good(id: &str) -> Option<&'static GoodDef> {
    GOODS.iter().find(|g| g.id == id)
}

pub fn elements() -> impl Iterator<Item = &'static GoodDef> {
    GOODS.iter().filter(|g| g.kind == GoodKind::Element)
}

pub fn products() -> impl Iterator<Item = &'static GoodDef> {
    GOODS.iter().filter(|g| g.kind == GoodKind::Product)
}

#[derive(Clone, Copy, Debug)]
pub struct BomIo {
    pub id: &'static str,
    pub qty: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct BomRecipe {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub inputs: &'static [BomIo],
    pub outputs: &'static [BomIo],
    /// Optional labor hours if not already in inputs (0 = none extra)
    pub labor_note: &'static str,
}

/// BOM chains — production loop gamifier (transform, not invent orgs).
pub const BOMS: &[BomRecipe] = &[
    BomRecipe {
        id: "cut_wood",
        name: "CUT WOOD",
        blurb: "Labor + energy → timber.",
        inputs: &[
            BomIo {
                id: "labor",
                qty: 2.0,
            },
            BomIo {
                id: "energy",
                qty: 1.0,
            },
        ],
        outputs: &[BomIo {
            id: "wood",
            qty: 1.0,
        }],
        labor_note: "field cut",
    },
    BomRecipe {
        id: "pump_water",
        name: "PUMP WATER",
        blurb: "Energy + labor → stored water.",
        inputs: &[
            BomIo {
                id: "energy",
                qty: 2.0,
            },
            BomIo {
                id: "labor",
                qty: 0.5,
            },
        ],
        outputs: &[BomIo {
            id: "water",
            qty: 1.0,
        }],
        labor_note: "pump run",
    },
    BomRecipe {
        id: "grow_food",
        name: "GROW FOOD",
        blurb: "Water + labor + energy → raw food.",
        inputs: &[
            BomIo {
                id: "water",
                qty: 1.0,
            },
            BomIo {
                id: "labor",
                qty: 3.0,
            },
            BomIo {
                id: "energy",
                qty: 1.0,
            },
        ],
        outputs: &[BomIo {
            id: "food_raw",
            qty: 2.0,
        }],
        labor_note: "garden",
    },
    BomRecipe {
        id: "cook_meal",
        name: "COOK MEAL",
        blurb: "Raw food + energy + labor → meal.",
        inputs: &[
            BomIo {
                id: "food_raw",
                qty: 1.0,
            },
            BomIo {
                id: "energy",
                qty: 1.5,
            },
            BomIo {
                id: "labor",
                qty: 0.5,
            },
        ],
        outputs: &[BomIo {
            id: "meal",
            qty: 1.0,
        }],
        labor_note: "kitchen",
    },
    BomRecipe {
        id: "make_biochar",
        name: "MAKE BIOCHAR",
        blurb: "Wood + labor → biochar (burn to soil).",
        inputs: &[
            BomIo {
                id: "wood",
                qty: 1.0,
            },
            BomIo {
                id: "labor",
                qty: 2.0,
            },
            BomIo {
                id: "energy",
                qty: 0.5,
            },
        ],
        outputs: &[BomIo {
            id: "biochar",
            qty: 4.0,
        }],
        labor_note: "kiln",
    },
    BomRecipe {
        id: "bag_biochar",
        name: "BAG BIOCHAR",
        blurb: "Biochar → bagged product.",
        inputs: &[
            BomIo {
                id: "biochar",
                qty: 5.0,
            },
            BomIo {
                id: "labor",
                qty: 0.5,
            },
        ],
        outputs: &[BomIo {
            id: "biochar_bag",
            qty: 1.0,
        }],
        labor_note: "pack",
    },
    BomRecipe {
        id: "camp_kit",
        name: "ASSEMBLE CAMP KIT",
        blurb: "Wood + labor → camp kit.",
        inputs: &[
            BomIo {
                id: "wood",
                qty: 2.0,
            },
            BomIo {
                id: "labor",
                qty: 4.0,
            },
            BomIo {
                id: "energy",
                qty: 1.0,
            },
        ],
        outputs: &[BomIo {
            id: "camp_kit",
            qty: 1.0,
        }],
        labor_note: "yard",
    },
    BomRecipe {
        id: "cube_kit",
        name: "ASSEMBLE CUBE KIT",
        blurb: "Wood + fill + gravel + labor + energy → cube kit.",
        inputs: &[
            BomIo {
                id: "wood",
                qty: 3.0,
            },
            BomIo {
                id: "fill",
                qty: 2.0,
            },
            BomIo {
                id: "gravel",
                qty: 1.0,
            },
            BomIo {
                id: "labor",
                qty: 8.0,
            },
            BomIo {
                id: "energy",
                qty: 4.0,
            },
        ],
        outputs: &[BomIo {
            id: "cube_kit",
            qty: 1.0,
        }],
        labor_note: "build stack",
    },
    BomRecipe {
        id: "trail_kit",
        name: "ASSEMBLE TRAIL KIT",
        blurb: "Gravel + labor + energy → trail kit.",
        inputs: &[
            BomIo {
                id: "gravel",
                qty: 1.0,
            },
            BomIo {
                id: "labor",
                qty: 3.0,
            },
            BomIo {
                id: "energy",
                qty: 1.0,
            },
        ],
        outputs: &[BomIo {
            id: "trail_kit",
            qty: 1.0,
        }],
        labor_note: "base crew",
    },
    BomRecipe {
        id: "soft_night",
        name: "HOST SOFT NIGHT",
        blurb: "Meal + energy + labor → hospitality night.",
        inputs: &[
            BomIo {
                id: "meal",
                qty: 1.0,
            },
            BomIo {
                id: "energy",
                qty: 3.0,
            },
            BomIo {
                id: "labor",
                qty: 2.0,
            },
            BomIo {
                id: "bandwidth",
                qty: 1.0,
            },
        ],
        outputs: &[BomIo {
            id: "soft_night",
            qty: 1.0,
        }],
        labor_note: "host",
    },
    BomRecipe {
        id: "solar_charge",
        name: "SOLAR CHARGE",
        blurb: "Labor (panel attend) → energy.",
        inputs: &[BomIo {
            id: "labor",
            qty: 1.0,
        }],
        outputs: &[BomIo {
            id: "energy",
            qty: 5.0,
        }],
        labor_note: "array",
    },
    BomRecipe {
        id: "dig_fill",
        name: "DIG FILL",
        blurb: "Labor + energy → fill earth.",
        inputs: &[
            BomIo {
                id: "labor",
                qty: 3.0,
            },
            BomIo {
                id: "energy",
                qty: 2.0,
            },
        ],
        outputs: &[BomIo {
            id: "fill",
            qty: 2.0,
        }],
        labor_note: "terrace",
    },
];

pub fn bom(id: &str) -> Option<&'static BomRecipe> {
    BOMS.iter().find(|b| b.id == id)
}

fn needs_of(recipe: &BomRecipe) -> Vec<(String, f64)> {
    recipe
        .inputs
        .iter()
        .map(|i| (i.id.to_string(), i.qty))
        .collect()
}

/// Run BOM once: consume inputs, mint outputs, log intent.
pub fn run_bom(recipe_id: &str) -> Result<String, String> {
    ensure_economy_boot();
    let recipe = bom(recipe_id).ok_or_else(|| "unknown recipe".to_string())?;
    let needs = needs_of(recipe);
    if !stock_has(&needs) {
        let missing: Vec<String> = needs
            .iter()
            .filter(|(id, q)| stock_qty(id) + 1e-9 < *q)
            .map(|(id, q)| format!("{id} need {q} have {:.1}", stock_qty(id)))
            .collect();
        return Err(format!("missing: {}", missing.join(", ")));
    }
    if !stock_consume(&needs) {
        return Err("consume failed".into());
    }
    for o in recipe.outputs {
        stock_add(o.id, o.qty);
    }
    let out_s: Vec<String> = recipe
        .outputs
        .iter()
        .map(|o| format!("+{} {}", o.qty, o.id))
        .collect();
    push_intent(
        "YOU",
        "bom",
        &format!("{} → {}", recipe.id, out_s.join(" ")),
    );
    let mut b = load_balance();
    b.depth = (b.depth + 1.5).min(99.0);
    crate::wallet::save_balance(&b);
    Ok(format!("{} · {}", recipe.name, out_s.join(", ")))
}

/// Buy from a sell order (city seed or peer).
pub fn market_buy(order_id: u64, qty: f64) -> Result<String, String> {
    ensure_economy_boot();
    if qty <= 0.0 {
        return Err("qty must be > 0".into());
    }
    let mut orders = load_orders();
    let idx = orders
        .iter()
        .position(|o| o.id == order_id && o.side == "sell")
        .ok_or_else(|| "order not found".to_string())?;
    let order = orders[idx].clone();
    let take = qty.min(order.qty);
    let cost = take * order.price_cx;
    let bal = load_balance();
    if bal.cx + 1e-9 < cost {
        return Err(format!("need {cost:.1} CX, have {:.1}", bal.cx));
    }
    debit_cx(cost);
    // seller credit if not seed
    if order.owner != "cyber-valley" {
        // peer: we only have one wallet in soft3 — credit CX only for seed asymmetry
        // peer sells: they already lost stock when listing; CX would need multi-wallet.
        // soft3 single-agent: seed is infinite-ish seller; peer sells from your list only to city buy-back later.
        credit_cx(0.0);
    }
    stock_add(&order.good_id, take);
    if take + 1e-9 >= order.qty {
        orders.remove(idx);
    } else {
        orders[idx].qty -= take;
    }
    save_orders(&orders);
    push_intent(
        "YOU",
        "buy",
        &format!("{} x{take} @ {:.2} CX", order.good_id, order.price_cx),
    );
    Ok(format!("bought {take} {} for {cost:.1} CX", order.good_id))
}

/// List a sell order from your stock.
pub fn market_list_sell(good_id: &str, qty: f64, price_cx: f64) -> Result<String, String> {
    ensure_economy_boot();
    if qty <= 0.0 || price_cx <= 0.0 {
        return Err("qty and price must be > 0".into());
    }
    if good(good_id).is_none() {
        return Err("unknown good".into());
    }
    if stock_qty(good_id) + 1e-9 < qty {
        return Err(format!(
            "have {:.1} {}, need {qty}",
            stock_qty(good_id),
            good_id
        ));
    }
    stock_add(good_id, -qty);
    let handle = load_profile().handle;
    let mut orders = load_orders();
    let id = next_order_id();
    orders.insert(
        0,
        MarketOrder {
            id,
            good_id: good_id.into(),
            qty,
            price_cx,
            side: "sell".into(),
            owner: handle.clone(),
        },
    );
    save_orders(&orders);
    push_intent(
        &handle,
        "list",
        &format!("sell {qty} {good_id} @ {price_cx} CX"),
    );
    Ok(format!("listed {qty} {good_id} @ {price_cx} CX"))
}

/// Cancel your sell order — return stock.
pub fn market_cancel(order_id: u64) -> Result<String, String> {
    let handle = load_profile().handle;
    let mut orders = load_orders();
    let idx = orders
        .iter()
        .position(|o| o.id == order_id && o.owner == handle)
        .ok_or_else(|| "not your order".to_string())?;
    let o = orders.remove(idx);
    if o.side == "sell" {
        stock_add(&o.good_id, o.qty);
    }
    save_orders(&orders);
    push_intent(&handle, "cancel", &format!("order #{}", o.id));
    Ok(format!("cancelled #{}", o.id))
}

/// City buy-back: sell into seed at 70% of lowest seed ask or floor.
pub fn market_sell_to_city(good_id: &str, qty: f64) -> Result<String, String> {
    ensure_economy_boot();
    if qty <= 0.0 {
        return Err("qty must be > 0".into());
    }
    if stock_qty(good_id) + 1e-9 < qty {
        return Err("insufficient stock".into());
    }
    let asks: Vec<f64> = load_orders()
        .into_iter()
        .filter(|o| o.good_id == good_id && o.side == "sell" && o.owner == "cyber-valley")
        .map(|o| o.price_cx)
        .collect();
    let px = asks
        .into_iter()
        .fold(None, |a: Option<f64>, b| {
            Some(match a {
                Some(x) => x.min(b),
                None => b,
            })
        })
        .map(|p| p * 0.70)
        .unwrap_or(1.0);
    stock_add(good_id, -qty);
    let rev = qty * px;
    credit_cx(rev);
    push_intent(
        "YOU",
        "sell",
        &format!("{good_id} x{qty} → city @ {px:.2} CX"),
    );
    Ok(format!("sold {qty} {good_id} to city for {rev:.1} CX"))
}

pub fn fmt_qty(q: f64) -> String {
    if (q - q.round()).abs() < 1e-6 {
        format!("{:.0}", q)
    } else if q >= 10.0 {
        format!("{q:.1}")
    } else {
        format!("{q:.2}")
    }
}
