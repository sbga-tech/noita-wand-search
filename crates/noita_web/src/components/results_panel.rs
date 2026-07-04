use leptos::prelude::*;
use noita_sim::data::{Spell, WAND_SPRITES};
use noita_sim::search::{SearchHit, SearchMode};
use noita_sim::types::Wand;
use noita_sim::ActionType;

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

/// Human-readable wand sprite/staff name for the card heading.
fn wand_name(wand: &Wand) -> &'static str {
    WAND_SPRITES
        .get(wand.sprite)
        .map(|sprite| sprite.name)
        .unwrap_or("Wand")
}

/// Inventory frame (`ui_gfx/inventory/item_bg_*`) overlaid on a spell icon,
/// picked by the spell's [`ActionType`] — the same frames the game draws behind
/// inventory items.
fn frame_asset(spell: Spell) -> &'static str {
    match spell.action_type() {
        ActionType::Projectile => "item_bg_projectile",
        ActionType::StaticProjectile => "item_bg_static_projectile",
        ActionType::Modifier => "item_bg_modifier",
        ActionType::DrawMany => "item_bg_draw_many",
        ActionType::Material => "item_bg_material",
        ActionType::Other => "item_bg_other",
        ActionType::Utility => "item_bg_utility",
        ActionType::Passive => "item_bg_passive",
    }
}

/// A single spell card: the gun-action glyph with its type frame overlaid. In an
/// inventory slot (`boxed`) it sits on the `inventory_box` background — an empty
/// slot renders just that box; the always-cast permanent action is not a slot,
/// so it is drawn unboxed.
#[component]
fn SpellCard(spell: Spell, #[prop(default = true)] boxed: bool) -> impl IntoView {
    let card_class = if boxed {
        "relative flex h-[42px] w-[42px] items-center justify-center bg-[url(public/assets/inventory/inventory_box.png)] bg-[length:100%_100%] bg-center bg-no-repeat [image-rendering:pixelated]"
    } else {
        "relative flex h-[42px] w-[42px] items-center justify-center [image-rendering:pixelated]"
    };
    let icon = spell.icon();
    if spell == Spell::None || icon.is_empty() {
        return view! { <div class=card_class></div> }.into_any();
    }
    let name = spell.display_name("en").to_string();
    let frame = frame_asset(spell);
    view! {
        <div class=card_class title=name.clone()>
            <img
                class="relative z-[1] h-8 w-8 [image-rendering:pixelated]"
                src=format!("public/assets/gun_actions/{icon}.png")
                alt=name.clone()
                loading="lazy"
            />
            <img
                class="pointer-events-none absolute inset-0 z-[2] h-full w-full [image-rendering:pixelated]"
                src=format!("public/assets/inventory/{frame}.png")
                alt=""
                aria-hidden="true"
            />
        </div>
    }
    .into_any()
}

/// One inventory-icon stat row: an icon+label cell and a value cell (two direct
/// grid children of the wand card).
#[component]
fn StatRow(icon: &'static str, label: &'static str, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="stat-label">
            <img src=format!("public/assets/inventory/{icon}.png") alt="" />
            {label}
        </div>
        <div class="stat-value">{value}</div>
    }
}

/// Always-cast row: a permanent-actions label and the framed spell icon.
#[component]
fn AlwaysCastRow(spell: Spell) -> impl IntoView {
    view! {
        <div class="stat-label mt-[0.6rem]">
            <img src="public/assets/inventory/icon_gun_permanent_actions.png" alt="" />
            "Always casts"
        </div>
        <div class="stat-value mt-[0.6rem] flex items-center gap-1">
            <SpellCard spell boxed=false />
        </div>
    }
}

/// A wand rendered as a Noita-style card matching the game/wiki layout.
#[component]
fn WandCard(wand: Wand) -> impl IntoView {
    let has_always_cast = wand.always_cast != Spell::None;

    // Grid areas: the sprite spans the name row plus every stat row so it sits
    // beside the ledger, exactly like the wiki template.
    let stat_rows = 9 + usize::from(has_always_cast);
    let mut areas = String::from("'name name image'");
    for _ in 0..stat_rows {
        areas.push_str(" 'label value image'");
    }
    areas.push_str(" 'spells spells spells'");

    let capacity = wand.capacity.max(0) as usize;
    let slots = capacity.max(wand.spells.len());
    let deck = (0..slots)
        .map(|i| wand.spells.get(i).copied().unwrap_or(Spell::None))
        .collect::<Vec<_>>();

    let name = wand_name(&wand).to_uppercase();

    view! {
        <div class="wand-card" style=format!("grid-template-areas: {areas};")>
            <p class="wand-name">{name.clone()}</p>
            <div class="wand-sprite">
                <img src=format!("public/assets/wands/wand_{:04}.png", wand.sprite) alt=name />
            </div>
            <StatRow icon="icon_gun_shuffle" label="Shuffle" value=if wand.shuffle { "Yes" } else { "No" } />
            <StatRow icon="icon_gun_actions_per_round" label="Spells/Cast" value=wand.multicast.to_string() />
            <StatRow icon="icon_fire_rate_wait" label="Cast delay" value=format!("{:.2} s", wand.delay) />
            <StatRow icon="icon_gun_reload_time" label="Rechrg. Time" value=format!("{:.2} s", wand.reload) />
            <StatRow icon="icon_mana_max" label="Mana max" value=wand.mana.to_string() />
            <StatRow icon="icon_mana_charge_speed" label="Mana chg. Spd" value=wand.regen.to_string() />
            <StatRow icon="icon_gun_capacity" label="Capacity" value=wand.capacity.to_string() />
            <StatRow icon="icon_spread_degrees" label="Spread" value=format!("{:.1} DEG", wand.spread as f64) />
            <StatRow icon="icon_speed_multiplier" label="Speed" value=format!("\u{00d7}\u{00a0}{:.2}", wand.speed) />
            <Show when=move || has_always_cast>
                <AlwaysCastRow spell=wand.always_cast />
            </Show>
            <div class="mt-4 flex flex-wrap gap-[0.3rem] [grid-area:spells]">
                <For each=move || deck.clone().into_iter().enumerate() key=|(i, _)| *i let:entry>
                    <SpellCard spell=entry.1 />
                </For>
            </div>
        </div>
    }
}

/// A found-wand result: title, coordinates, and the wand card.
#[component]
fn WandHit(mode: SearchMode, hit: SearchHit) -> impl IntoView {
    let SearchHit::Wand { x, y, wand, .. } = hit;
    let suffix = uncertainty_suffix(mode, x, y);
    view! {
        <div class="mb-1 font-display text-xl uppercase tracking-widest text-gold-bright">"Wand found"</div>
        <div class="mb-3 tabular-nums text-parchment">{format!("x = {x}{suffix} · y = {y}{suffix}")}</div>
        <WandCard wand />
    }
}

#[component]
pub fn ResultsPanel(
    status: ReadSignal<String>,
    error: ReadSignal<String>,
    result: ReadSignal<Option<SearchHit>>,
    mode: ReadSignal<SearchMode>,
) -> impl IntoView {
    let body = move || {
        let error = error.get();
        if !error.is_empty() {
            view! { <div class="border-2 border-blood bg-[rgba(194,55,29,0.15)] p-4 text-center text-[#ffb3a6]">{error}</div> }.into_any()
        } else if let Some(hit) = result.get() {
            view! { <WandHit mode=mode.get() hit /> }.into_any()
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

    fn sample_wand() -> Wand {
        Wand {
            capacity: 3,
            multicast: 2,
            mana: 500,
            regen: 250,
            delay: 0.32,
            reload: 0.55,
            speed: 1.0,
            spread: -3,
            shuffle: false,
            always_cast: Spell::Homing,
            sprite: 821,
            spells: [Spell::ManaReduce, Spell::CircleshotA]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn frame_asset_follows_action_type() {
        // Projectile spell -> projectile frame.
        assert_eq!(frame_asset(Spell::CircleshotA), "item_bg_projectile");
        // Modifier spell -> modifier frame (the reference asset).
        assert_eq!(frame_asset(Spell::ManaReduce), "item_bg_modifier");
        assert_eq!(frame_asset(Spell::Homing), "item_bg_modifier");
        // Material spell -> material frame.
        assert_eq!(frame_asset(Spell::Soilball), "item_bg_material");
    }

    #[test]
    fn spell_action_types_resolve() {
        assert_eq!(Spell::Mana.action_type(), ActionType::Projectile);
        assert_eq!(Spell::ManaReduce.action_type(), ActionType::Modifier);
        assert_eq!(
            Spell::BlackHoleBig.action_type(),
            ActionType::StaticProjectile
        );
    }

    #[test]
    fn wand_name_resolves_sprite() {
        let wand = sample_wand();
        assert_eq!(wand_name(&wand), WAND_SPRITES[821].name);
    }

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
