use crate::components::ui::WandCard;
use leptos::prelude::*;
use noita_sim::loot::{
    find_wand_sprite, SpawnCoord, GREAT_CHEST_LOOT_TABLE, TAIKASAUVA_LOOT_TABLE,
    TINY_DROP_LOOT_TABLE,
};
use noita_sim::search::{SearchHit, SearchMode, SearchRequest};
use noita_sim::types::Wand;

pub fn heading(_mode: SearchMode) -> &'static str {
    "Result"
}

pub fn uncertainty_suffix(mode: SearchMode, x: f64, y: f64) -> &'static str {
    if mode == SearchMode::EoeWand && (x.abs() > 10_000_000.0 || y.abs() > 10_000_000.0) {
        " ± 50"
    } else if mode == SearchMode::EoeWand && (x.abs() > 1_000_000.0 || y.abs() > 1_000_000.0) {
        " ± 5"
    } else {
        ""
    }
}

fn sprite_for_hit(request: &SearchRequest, x: f64, y: f64, wand: &Wand) -> Option<usize> {
    let table = match request.mode {
        SearchMode::EoeWand => &GREAT_CHEST_LOOT_TABLE,
        SearchMode::TaikasauvaWand => &TAIKASAUVA_LOOT_TABLE,
        SearchMode::TinyDropWand => &TINY_DROP_LOOT_TABLE,
    };
    let save_flags = match (request.mode, request.unlock_flags.clone()) {
        (SearchMode::EoeWand, Some(flags)) => Some(noita_sim::SaveFlags::new(flags)),
        (SearchMode::EoeWand, None) => Some(noita_sim::SaveFlags::new(Vec::new())),
        (_, Some(flags)) => Some(noita_sim::SaveFlags::new(flags)),
        (_, None) => None,
    };
    find_wand_sprite(
        request.seed.wrapping_add(request.ng),
        table,
        save_flags.as_ref(),
        SpawnCoord {
            x: x as i32,
            y: y as i32,
        },
        wand,
    )
}

/// A found-wand result: title, coordinates, and the wand card.
#[component]
fn WandHit(mode: SearchMode, request: Option<SearchRequest>, hit: SearchHit) -> impl IntoView {
    let SearchHit::Wand { x, y, wand, .. } = hit;
    let sprite = request
        .as_ref()
        .and_then(|request| sprite_for_hit(request, x, y, &wand))
        .unwrap_or(0);
    let suffix = uncertainty_suffix(mode, x, y);
    view! {
        <div class="mb-1 font-display text-xl uppercase tracking-widest text-gold-bright">"Wand found"</div>
        <div class="mb-3 tabular-nums text-parchment">{format!("x = {x}{suffix} · y = {y}{suffix}")}</div>
        <WandCard wand sprite />
    }
}

#[component]
pub fn ResultsPanel(
    status: ReadSignal<String>,
    error: ReadSignal<String>,
    result: ReadSignal<Option<SearchHit>>,
    mode: ReadSignal<SearchMode>,
    request: ReadSignal<Option<SearchRequest>>,
) -> impl IntoView {
    let body = move || {
        let error = error.get();
        if !error.is_empty() {
            view! { <div class="border-2 border-blood bg-[rgba(194,55,29,0.15)] p-4 text-center text-[#ffb3a6]">{error}</div> }.into_any()
        } else if let Some(hit) = result.get() {
            view! { <WandHit mode=mode.get() request=request.get() hit /> }.into_any()
        } else {
            view! {
                <div class="border-2 border-dashed border-bronze p-4 text-center text-parchment-dim">"Complete a search to reveal the wand info."</div>
            }
            .into_any()
        }
    };
    view! {
        <section id="output_box" class="panel results">
            <div class="panel-head">
                <h2 class="panel-title">{move || heading(mode.get())}</h2>
                <span id="status">{move || status.get()}</span>
            </div>
            <div id="output">{body}</div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertainty_scales_with_distance() {
        assert_eq!(uncertainty_suffix(SearchMode::EoeWand, 0.0, 0.0), "");
        assert_eq!(
            uncertainty_suffix(SearchMode::EoeWand, 2_000_000.0, 0.0),
            " ± 5"
        );
        assert_eq!(
            uncertainty_suffix(SearchMode::EoeWand, 20_000_000.0, 0.0),
            " ± 50"
        );
    }
}
