mod admin;
mod app;
mod cabinet;
mod cities;
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
mod land;
mod me;
mod nav;
mod orders;
mod pay;
mod places;
mod plot;
mod plot_board;
mod plot_docs;
mod plot_ground;
mod plot_pay;
mod plot_land;
mod portal;
mod products;
mod plots;
mod orgs;
mod robots;
mod services;
mod signal;
mod signal_pages;
mod states;
mod studio;
mod terms;
mod wallet;
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
