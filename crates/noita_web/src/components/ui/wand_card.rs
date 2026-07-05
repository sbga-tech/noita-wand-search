use super::SpellCard;
use leptos::prelude::*;
use noita_sim::data::{Spell, WAND_SPRITES};
use noita_sim::types::Wand;

/// Human-readable wand sprite/staff name for the card heading.
fn wand_name(sprite: usize) -> &'static str {
    WAND_SPRITES
        .get(sprite)
        .map(|sprite| sprite.name)
        .unwrap_or("Wand")
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
pub fn WandCard(wand: Wand, sprite: usize) -> impl IntoView {
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

    let name = wand_name(sprite).to_uppercase();

    view! {
        <div class="wand-card" style=format!("grid-template-areas: {areas};")>
            <p class="wand-name">{name.clone()}</p>
            <div class="wand-sprite">
                <img src=format!("public/assets/wands/wand_{sprite:04}.png") alt=name />
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wand_name_resolves_sprite() {
        assert_eq!(wand_name(821), WAND_SPRITES[821].name);
    }
}
