use crate::components::control_panel::{validate_state, ControlPanel, FormState};
use crate::components::map_panel::MapPanel;
use crate::components::results_panel::ResultsPanel;
use crate::components::search_runner::{cancel_active_search_worker, spawn_client_search};
use leptos::prelude::*;
use noita_sim::search::{SearchMode, SearchProgress};

#[component]
pub fn App() -> impl IntoView {
    let status = RwSignal::new("Ready.".to_string());
    let error = RwSignal::new(String::new());
    let result = RwSignal::new(None);
    let mode = RwSignal::new(SearchMode::EoeWand);
    let ng = RwSignal::new(0_u32);
    let active_token = RwSignal::new(0_u64);
    let searching = RwSignal::new(false);
    let progress = RwSignal::new(SearchProgress {
        x: 0.0,
        y: 0.0,
        searched_pixels: 0,
    });

    let on_search = Callback::new(move |state: FormState| {
        let request = match validate_state(&state) {
            Ok(request) => request,
            Err(message) => {
                active_token.update(|token| *token = token.wrapping_add(1));
                searching.set(false);
                status.set("Ready.".to_string());
                error.set(message);
                result.set(None);
                return;
            }
        };
        let token = active_token.with_untracked(|token| token.wrapping_add(1));
        active_token.set(token);
        searching.set(true);
        status.set("Searching...".to_string());
        error.set(String::new());
        result.set(None);
        mode.set(request.mode);
        ng.set(request.ng);
        progress.set(SearchProgress {
            x: request.start_x,
            y: request.start_y,
            searched_pixels: 0,
        });
        spawn_client_search(
            request,
            token,
            active_token.read_only(),
            status.write_only(),
            result.write_only(),
            progress.write_only(),
            searching.write_only(),
        );
    });

    let on_cancel = Callback::new(move |()| {
        active_token.update(|token| *token = token.wrapping_add(1));
        cancel_active_search_worker();
        searching.set(false);
        status.set("Cancelled.".to_string());
    });

    view! {
        <div class="background-dim"></div>
        <main class="atlas-shell">
            <header class="mb-6 flex items-center">
                <img class="h-16 w-auto [image-rendering:pixelated]" src="public/site-icon.png" alt="" />
                <div>
                    <p class="m-0 mt-2 max-w-[70ch] font-display text-sm uppercase tracking-[0.34em] text-parchment-dim">"Arcane cartographer's atlas"</p>
                    <h1 class="atlas-title">"Noita Wand Search"</h1>
                </div>
            </header>
            <div class="atlas-grid">
                <div class="left-column">
                    <ControlPanel on_search on_cancel searching=searching.read_only() />
                    <ResultsPanel status=status.read_only() error=error.read_only() result=result.read_only() mode=mode.read_only() />
                </div>
                <div class="right-column">
                    <MapPanel ng=ng.read_only() progress=progress.read_only() status=status.read_only() />
                    <section class="panel description">
                        <p>"Fully in-browser wand search tool written in rust. Support multi-predicate filtering and goes FAST on web workers thread pool."<br/> <a class="block text-right" href="https://github.com/sbga-tech/noita-wand-search">"Github"</a></p>
                    </section>
                </div>
            </div>
        </main>
    }
}
