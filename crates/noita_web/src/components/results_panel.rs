use leptos::prelude::*;
use noita_sim::data::{Spell, WAND_SPRITES};
use noita_sim::search::{SearchHit, SearchMode};
use noita_sim::types::Wand;

pub fn heading(_mode: SearchMode) -> &'static str {
    "Wands"
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

fn deck_names(wand: &Wand) -> Vec<&'static str> {
    let capacity = wand.capacity.max(0) as usize;
    let mut names = wand
        .spells
        .iter()
        .map(|spell| spell.display_name("en"))
        .collect::<Vec<_>>();
    names.resize(capacity.max(names.len()), "None");
    names
}

pub fn format_wand(wand: &Wand) -> String {
    let deck = deck_names(wand).join(", ");
    format!("capacity: {}; multicast: {}; cast delay: {:.2}s; reload: {:.2}s; max mana: {}; mana regen: {}; spread: {}°; speed: {:.3}x; {}; always-cast: {}; sprite: {}; deck: [{}]",
        wand.capacity, wand.multicast, wand.delay, wand.reload, wand.mana, wand.regen, wand.spread, wand.speed, if wand.shuffle { "shuffle" } else { "nonshuffle" }, wand.always_cast.display_name("en"), wand.sprite, deck)
}

pub fn format_hit(mode: SearchMode, hit: &SearchHit) -> String {
    let SearchHit::Wand { x, y, wand, .. } = hit;
    format!(
        "Wand found at x = {x}{}, y = {y}{}. {}",
        uncertainty_suffix(mode, *x, *y),
        uncertainty_suffix(mode, *x, *y),
        format_wand(wand)
    )
}

/// Minimal HTML escaping for text interpolated into `inner_html`.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Human-readable wand sprite/staff name for the card heading.
fn wand_name(wand: &Wand) -> &'static str {
    WAND_SPRITES
        .get(wand.sprite)
        .map(|sprite| sprite.name)
        .unwrap_or("Wand")
}

/// One inventory-icon stat row: an icon+label cell and a value cell.
fn stat_row(icon: &str, label: &str, value: &str) -> String {
    format!(
        "<div class=\"stat-label\"><img src=\"/public/assets/inventory/{icon}.png\" alt=\"\" />{label}</div><div class=\"stat-value\">{value}</div>"
    )
}

/// A single spell slot: the gun-action icon on a dashed frame, or, for
/// [`Spell::None`], an empty frame (matching an unfilled capacity slot).
fn spell_box(spell: Spell) -> String {
    let icon = spell.icon();
    if spell == Spell::None || icon.is_empty() {
        return "<div class=\"wand-card-spell\"></div>".to_string();
    }
    let name = escape(spell.display_name("en"));
    format!(
        "<div class=\"wand-card-spell\" title=\"{name}\"><img src=\"/public/assets/gun_actions/{icon}.png\" alt=\"{name}\" loading=\"lazy\" /></div>"
    )
}

/// The always-cast row rendered like the wiki's permanent-actions row: a small
/// spell icon rather than a stat value.
fn always_cast_row(spell: Spell) -> String {
    let icon = spell.icon();
    let name = escape(spell.display_name("en"));
    let inner = if icon.is_empty() {
        format!("<span>{name}</span>")
    } else {
        format!("<img src=\"/public/assets/gun_actions/{icon}.png\" alt=\"{name}\" title=\"{name}\" loading=\"lazy\" />")
    };
    format!(
        "<div class=\"stat-label always-cast-label\"><img src=\"/public/assets/inventory/icon_gun_permanent_actions.png\" alt=\"\" />Always casts</div><div class=\"stat-value always-cast-spells\">{inner}</div>"
    )
}

/// Renders a wand as a Noita-style wand card matching the game/wiki layout.
fn wand_html(wand: &Wand) -> String {
    let mut rows = vec![
        stat_row(
            "icon_gun_shuffle",
            "Shuffle",
            if wand.shuffle { "Yes" } else { "No" },
        ),
        stat_row(
            "icon_gun_actions_per_round",
            "Spells/Cast",
            &wand.multicast.to_string(),
        ),
        stat_row(
            "icon_fire_rate_wait",
            "Cast delay",
            &format!("{:.2} s", wand.delay),
        ),
        stat_row(
            "icon_gun_reload_time",
            "Rechrg. Time",
            &format!("{:.2} s", wand.reload),
        ),
        stat_row("icon_mana_max", "Mana max", &wand.mana.to_string()),
        stat_row(
            "icon_mana_charge_speed",
            "Mana chg. Spd",
            &wand.regen.to_string(),
        ),
        stat_row("icon_gun_capacity", "Capacity", &wand.capacity.to_string()),
        stat_row(
            "icon_spread_degrees",
            "Spread",
            &format!("{:.1} DEG", wand.spread as f64),
        ),
        stat_row(
            "icon_speed_multiplier",
            "Speed",
            &format!("\u{00d7}\u{00a0}{:.2}", wand.speed),
        ),
    ];
    if wand.always_cast != Spell::None {
        rows.push(always_cast_row(wand.always_cast));
    }

    // Grid areas: the sprite spans the name row plus every stat row so it sits
    // centered beside the ledger, exactly like the wiki template.
    let mut areas = String::from("'name name image'");
    for _ in &rows {
        areas.push_str(" 'label value image'");
    }
    areas.push_str(" 'spells spells spells'");

    let capacity = wand.capacity.max(0) as usize;
    let slots = capacity.max(wand.spells.len());
    let spells = (0..slots)
        .map(|i| spell_box(wand.spells.get(i).copied().unwrap_or(Spell::None)))
        .collect::<String>();

    format!(
        "<div class=\"wand-card\" style=\"grid-template-areas: {areas};\">\
           <p class=\"wand-name\">{name}</p>\
           <div class=\"wand-sprite\"><img src=\"/public/assets/wands/wand_{sprite:04}.png\" alt=\"{name}\" /></div>\
           {rows}\
           <div class=\"spell-container\">{spells}</div>\
         </div>",
        name = escape(&wand_name(wand).to_uppercase()),
        sprite = wand.sprite,
        rows = rows.concat(),
    )
}

pub fn format_hit_html(mode: SearchMode, hit: &SearchHit) -> String {
    let SearchHit::Wand { x, y, wand, .. } = hit;
    format!(
        "<div class=\"hit-title\">Wand found</div><div class=\"coords\">x = {x}{} · y = {y}{}</div>{}",
        uncertainty_suffix(mode, *x, *y),
        uncertainty_suffix(mode, *x, *y),
        wand_html(wand)
    )
}

#[component]
pub fn ResultsPanel(
    status: ReadSignal<String>,
    error: ReadSignal<String>,
    result: ReadSignal<Option<SearchHit>>,
    mode: ReadSignal<SearchMode>,
) -> impl IntoView {
    let output_html = move || {
        if !error.get().is_empty() {
            format!("<div class=\"error\">{}</div>", error.get())
        } else if let Some(hit) = result.get() {
            format_hit_html(mode.get(), &hit)
        } else {
            "<div class=\"empty-result\">Submit a search to reveal the wand ledger.</div>"
                .to_string()
        }
    };
    view! {
        <section id="output_box" class="panel results">
            <div class="panel-head"><h2 class="panel-title">{move || heading(mode.get())}</h2><span id="status">{move || status.get()}</span></div>
            <div id="output" inner_html=output_html></div>
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
            spells: vec![Spell::ManaReduce, Spell::CircleshotA],
        }
    }

    #[test]
    fn card_uses_game_assets_and_wiki_structure() {
        let html = wand_html(&sample_wand());
        assert!(html.contains("class=\"wand-card\""));
        // Sprite index -> zero-padded wand file.
        assert!(html.contains("/public/assets/wands/wand_0821.png"));
        // Inventory stat icon + label/value cells.
        assert!(html.contains("/public/assets/inventory/icon_gun_shuffle.png"));
        assert!(html.contains("<div class=\"stat-value\">No</div>"));
        assert!(html.contains("0.32 s"));
        assert!(html.contains("\u{00d7}\u{00a0}1.00"));
        // Spell icons resolve via Spell::icon (divergent basename included).
        assert!(html.contains("/public/assets/gun_actions/mana.png"));
        assert!(html.contains("/public/assets/gun_actions/phantomshot_a.png"));
    }

    #[test]
    fn always_cast_row_only_when_present() {
        let with = wand_html(&sample_wand());
        assert!(with.contains("icon_gun_permanent_actions.png"));
        assert!(with.contains("/public/assets/gun_actions/homing.png"));

        let mut plain = sample_wand();
        plain.always_cast = Spell::None;
        let without = wand_html(&plain);
        assert!(!without.contains("icon_gun_permanent_actions.png"));
    }

    #[test]
    fn empty_capacity_slots_render_as_blank_boxes() {
        // capacity 3, two spells -> three slots, one empty.
        let html = wand_html(&sample_wand());
        assert_eq!(html.matches("class=\"wand-card-spell\"").count(), 3);
        assert!(html.contains("<div class=\"wand-card-spell\"></div>"));
    }
}
