use leptos::prelude::*;
use noita_sim::{ActionType, Spell};

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
/// slot renders just that box; permanent actions can be drawn unboxed.
#[component]
pub fn SpellCard(
    spell: Spell,
    #[prop(default = true)] boxed: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    let boxed_class = if boxed {
        "bg-[url(public/assets/inventory/inventory_box.png)] bg-[length:100%_100%] bg-center bg-no-repeat"
    } else {
        ""
    };
    let card_class = format!(
        "relative flex h-[42px] w-[42px] items-center justify-center overflow-hidden {boxed_class} [image-rendering:pixelated] {class}"
    );
    let icon = spell.icon();
    if spell == Spell::None || icon.is_empty() {
        return view! { <div class=card_class></div> }.into_any();
    }
    let name = spell.display_name("en").to_string();
    let frame = frame_asset(spell);
    view! {
        <div class=card_class title=name.clone()>
            <img
                class="relative z-[2] h-full w-full scale-[0.7] [image-rendering:pixelated]"
                src=format!("public/assets/gun_actions/{icon}.png")
                alt=name.clone()
                loading="lazy"
            />
            <img
                class="pointer-events-none absolute inset-0 z-[1] h-full w-full [image-rendering:pixelated]"
                src=format!("public/assets/inventory/{frame}.png")
                alt=""
                aria-hidden="true"
                loading="lazy"
            />
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_asset_follows_action_type() {
        assert_eq!(frame_asset(Spell::CircleshotA), "item_bg_projectile");
        assert_eq!(frame_asset(Spell::ManaReduce), "item_bg_modifier");
        assert_eq!(frame_asset(Spell::Homing), "item_bg_modifier");
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
}
