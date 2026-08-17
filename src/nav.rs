//! Shared top nav for cyberia.my catalog surfaces.

use leptos::prelude::*;

/// `active`: you | cities | elements | market | calendar | robots | plots | places | map | states
#[component]
pub fn CyberiaNav(#[prop(into)] active: String) -> impl IntoView {
    let a = active;
    view! {
        <div class="map-zone cyberia-nav">
            <a class=if a == "you" { "nav-btn nav-here nav-you" } else { "nav-btn nav-you" } href="/me">"YOU"</a>
            <a class=if a == "world" { "nav-btn nav-here" } else { "nav-btn" } href="/world">"WORLD"</a>
            <a class=if a == "studio" { "nav-btn nav-here" } else { "nav-btn" } href="/studio">"STUDIO"</a>
            <a class=if a == "cities" { "nav-btn nav-here" } else { "nav-btn" } href="/cities">"CITIES"</a>
            <a class=if a == "elements" { "nav-btn nav-here" } else { "nav-btn" } href="/elements">"ELEMENTS"</a>
            <a class=if a == "products" { "nav-btn nav-here" } else { "nav-btn" } href="/products">"PRODUCTS"</a>
            <a class=if a == "genetics" { "nav-btn nav-here" } else { "nav-btn" } href="/genetics">"GENETICS"</a>
            <a class=if a == "services" { "nav-btn nav-here" } else { "nav-btn" } href="/services">"SERVICES"</a>
            <a class=if a == "orgs" { "nav-btn nav-here" } else { "nav-btn" } href="/orgs">"ORGS"</a>
            <a class=if a == "calendar" { "nav-btn nav-here" } else { "nav-btn" } href="/calendar">"CALENDAR"</a>
            <a class=if a == "robots" { "nav-btn nav-here" } else { "nav-btn" } href="/robots">"ROBOTS"</a>
            <a class=if a == "plots" { "nav-btn nav-here" } else { "nav-btn" } href="/plots">"PLOTS"</a>
            <a class=if a == "places" { "nav-btn nav-here" } else { "nav-btn" } href="/places">"PLACES"</a>
            <a class=if a == "map" { "nav-btn nav-here" } else { "nav-btn" } href="/map">"MAP"</a>
            <a class=if a == "domains" { "nav-btn nav-here" } else { "nav-btn" } href="/domains">"DOMAINS"</a>
            <a class=if a == "states" { "nav-btn nav-here" } else { "nav-btn" } href="/states">"STATES"</a>
        </div>
    }
}
