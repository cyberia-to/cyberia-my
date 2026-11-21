//! FLATS — the register of the land, on two levels: the title a flat is cut
//! from (`crate::titles`), and under it every flat with its use, its size,
//! the price in the lease book, where it stands, its holder and that
//! holder's wallet. A row opens in place: bind the holder, or go to the deal.

use crate::admin::{field, Desk};
use crate::deals::{flat_status, load_deals, FlatStatus};
use crate::globe::BASE;
use crate::land::{area_m2, load_map, LandFlat};
use crate::market::{short, View};
use crate::orders::PublishedDeal;
use crate::terms::{bare_name, fmt_usd, land_use, terms};
use leptos::prelude::*;

pub fn flats(d: Desk, v: &View) -> AnyView {
    let map = load_map();
    let mut local = load_deals();
    let mut index = d.bot.get();
    for x in &v.deals {
        index.insert(x.plot.clone(), PublishedDeal { status: x.status.clone(), holder: x.holder.clone(), paid_usd: x.paid_usd, ..Default::default() });
    }
    crate::deals::merge_published(&mut local, &index);
    let want = d.sub.get();
    let q = d.q.get().to_lowercase();
    let rows: Vec<(LandFlat, FlatStatus)> = map
        .phase0
        .iter()
        .map(|f| (f.clone(), flat_status(&f.id, &local)))
        .filter(|(f, st)| match want {
            1 => *st == FlatStatus::Available,
            2 => *st == FlatStatus::Transfer,
            3 => *st == FlatStatus::Owned,
            4 => *st == FlatStatus::OnRequest,
            _ => true,
        } && (q.is_empty() || f.id.contains(&q) || bare_name(&f.name).to_lowercase().contains(&q)))
        .collect();
    let n = rows.len();
    let vv = v.clone();
    let row = move |(f, st): (LandFlat, FlatStatus)| {
                let id = f.id.clone();
                let (id_c, id_o) = (id.clone(), id.clone());
                let book = terms(&id);
                let lu = land_use(&id);
                let deal = vv.deals.iter().find(|x| x.plot == id).cloned();
                let bound = vv.holders.iter().find(|h| h.plot == id).cloned();
                let offer = vv.listings.iter().any(|l| l.plot == id && l.status == "live");
                let holder = deal.as_ref().map(|x| x.holder.clone()).filter(|h| !h.is_empty())
                    .or_else(|| bound.as_ref().map(|h| h.name.clone()).filter(|h| !h.is_empty()))
                    .or_else(|| book.and_then(|t| t.holder()).map(|h| h.to_string()))
                    .unwrap_or_else(|| "—".to_string());
                let open = move || d.focus.get() == id_c;
                let has_deal = deal.is_some();
                view! {
                    <button type="button" class=move || if d.focus.get() == id_o { "ad-row on" } else { "ad-row" } on:click={ let id = id.clone(); move |_| d.look(&id) }>
                        <b>{bare_name(&f.name)}</b>
                        <span style:color=lu.color()>{lu.short()}</span>
                        <span>{format!("{:.1} ares", area_m2(&f.coords) / 100.0)}</span>
                        <span>{book.and_then(|t| t.price()).map(fmt_usd).unwrap_or_else(|| "—".into())}</span>
                        <span class=match st { FlatStatus::Available => "ad-st open", FlatStatus::Transfer => "ad-st deal", FlatStatus::Owned => "ad-st held", _ => "ad-st" }>{st.label()}{offer.then_some(" · OFFERED")}</span>
                        <span>{holder}</span>
                        <span class="mono">{bound.as_ref().map(|h| short(&h.address)).unwrap_or_else(|| "—".into())}</span>
                    </button>
                    {move || open().then(|| {
                        let id = id.clone();
                        let (id_d, id_h) = (id.clone(), id.clone());
                        view! {
                            <div class="ad-open">
                                <div class="dl-actions">
                                    <a class="dl-mini" href=format!("{BASE}/plot/{id}") target="_blank">"THE FLAT'S PAGE"</a>
                                    <a class="dl-mini" href=format!("{BASE}/map?plot={id}") target="_blank">"ON THE MAP"</a>
                                    <button type="button" class="dl-mini" on:click=move |_| { d.go(2, if has_deal { 0 } else { 2 }); d.focus.set(id_d.clone()); d.a.set(id_d.clone()); }>
                                        {if has_deal { "ITS DEAL →" } else { "OPEN A DEAL ON IT →" }}
                                    </button>
                                </div>
                                <div class="dl-q">"bind the flat to its holder's wallet"</div>
                                <div class="sl-grid">
                                    {field("the holder's address", "0x…", d.b)}
                                    {field("the name shown", "Ana", d.c)}
                                </div>
                                <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.b.get().trim().len() != 42
                                    on:click=move |_| d.sign("holder", vec![("plot", id_h.clone()), ("address", d.b.get_untracked().trim().to_string()), ("name", d.c.get_untracked())])>
                                    "SIGN · BIND THE HOLDER"
                                </button>
                            </div>
                        }
                    })}
                }
    };
    // the upper level: the parcel a flat is a piece of. a group is open when the person opened it,
    // or when a search or a filter already narrowed the list
    let narrowed = want != 0 || !q.is_empty();
    let mut groups: Vec<(Option<&'static crate::titles::Title>, Vec<(LandFlat, FlatStatus)>)> = Vec::new();
    for (f, st) in rows {
        let t = crate::titles::title_of(&f.id);
        match groups.iter_mut().find(|g| g.0.map(|x| &x.name) == t.map(|x| &x.name)) {
            Some(g) => g.1.push((f, st)),
            None => groups.push((t, vec![(f, st)])),
        }
    }
    groups.sort_by(|a, b| a.0.is_none().cmp(&b.0.is_none()).then(b.1.len().cmp(&a.1.len())));
    let n_titles = groups.iter().filter(|g| g.0.is_some()).count();
    view! {
        <div class="ad-bar">
            <input class="ad-search" type="text" placeholder="find a flat: sinwood-7, avalon…" prop:value=move || d.q.get() on:input=move |ev| d.q.set(event_target_value(&ev)) />
            <span class="ad-count">{format!("{n} flats on {n_titles} titles")}</span>
            <button type="button" class="dl-mini" on:click=move |_| d.groups.update(|g| if g.is_empty() { *g = crate::titles::book().titles.iter().map(|t| t.name.clone()).chain(["—".to_string()]).collect() } else { g.clear() })>
                {move || if d.groups.get().is_empty() { "OPEN ALL" } else { "CLOSE ALL" }}
            </button>
        </div>
        <div class="ad-table">
            {groups.into_iter().map(|(t, list)| {
                let key = t.map(|t| t.name.clone()).unwrap_or_else(|| "—".to_string());
                let (k1, k2) = (key.clone(), key.clone());
                let (mut open_n, mut deal_n, mut held_n, mut ares) = (0, 0, 0, 0.0);
                for (f, st) in &list {
                    match st { FlatStatus::Available => open_n += 1, FlatStatus::Transfer => deal_n += 1, FlatStatus::Owned => held_n += 1, _ => {} }
                    ares += area_m2(&f.coords) / 100.0;
                }
                let total = list.len();
                let row = row.clone();
                view! {
                    <button type="button" class=move || if narrowed || d.groups.get().contains(&k1) { "ad-title on" } else { "ad-title" }
                        on:click=move |_| d.groups.update(|g| if let Some(i) = g.iter().position(|x| x == &k2) { g.remove(i); } else { g.push(k2.clone()); })>
                        <span class="ad-title-n">
                            <b>{t.map(|t| t.name.to_uppercase()).unwrap_or_else(|| "NO TITLE ON THE MAP".to_string())}</b>
                            <i>{t.map(|t| t.held_line()).unwrap_or_else(|| "the map layer draws no parcel under these flats".to_string())}</i>
                        </span>
                        <span class="ad-title-f">{t.map(|t| t.facts()).unwrap_or_default()}</span>
                        <span class="ad-title-c">
                            <b>{format!("{total} flats · {ares:.0} ares")}</b>
                            <i>
                                {(open_n > 0).then(|| view! { <em class="open">{format!("{open_n} open")}</em> })}
                                {(deal_n > 0).then(|| view! { <em class="deal">{format!("{deal_n} in deal")}</em> })}
                                {(held_n > 0).then(|| view! { <em class="held">{format!("{held_n} held")}</em> })}
                            </i>
                        </span>
                    </button>
                    {move || (narrowed || d.groups.get().contains(&key)).then(|| view! {
                        <div class="ad-title-body">
                            <div class="ad-row head"><span>"flat"</span><span>"use"</span><span>"size"</span><span>"price in the book"</span><span>"stands"</span><span>"holder"</span><span>"wallet"</span></div>
                            {list.clone().into_iter().map(|x| row(x)).collect_view()}
                        </div>
                    })}
                }
            }).collect_view()}
        </div>
    }.into_any()
}
