mod app;
mod cities;
mod console;
mod economy;
mod elements;
mod erp;
mod events;
mod genetics;
mod land;
mod market;
mod me;
mod nav;
mod places;
mod products;
mod plots;
mod robots;
mod signal;
mod signal_pages;
mod states;
mod studio;
mod wallet;
mod world;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
