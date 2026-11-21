mod admin;
mod app;
mod cabinet;
mod console;
mod deals;
mod docs;
mod economy;
mod eth;
mod elements;
mod erp;
mod events;
mod friends;
mod genetics;
mod globe;
mod land;
mod market;
mod legend;
mod me;
mod my;
mod nav;
mod offers;
mod orders;
mod pay;
mod places;
mod plot;
mod plot_board;
mod aerial;
mod deal;
mod deal_close;
mod deal_docs;
mod deal_steps;
mod offer;
mod plot_files;
mod plot_land;
mod plot_money;
mod portal;
mod products;
mod plots;
mod orgs;
mod robots;
mod sell;
mod services;
mod signal;
mod signal_pages;
mod states;
mod studio;
mod terms;
mod titles;
mod wallet;
mod wallet_ui;
mod world;

fn main() {
    console_error_panic_hook::set_once();
    // a flat's static page (scripts/render-plot-pages.py) carries a summary
    // for crawlers — the app takes the screen from here
    if let Some(seo) = leptos::prelude::document().get_element_by_id("plot-seo") {
        seo.remove();
    }
    leptos::mount::mount_to_body(app::App);
}
