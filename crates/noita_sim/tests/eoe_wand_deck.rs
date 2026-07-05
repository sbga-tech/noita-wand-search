use noita_sim::filters::{WandFilter, WandFilterSet};
use noita_sim::loot::{find_wand_sprite, Item, LootSpawner, SpawnCoord, GREAT_CHEST_LOOT_TABLE};
use noita_sim::search::{SearchHit, SearchMode, SearchRequest, SearchState};
use noita_sim::types::Wand;
use noita_sim::wand::get_wand_unlocked;
use noita_sim::{SaveFlags, Spell};

const REPORTED_UNLOCK_FLAGS: &[&str] = &[
    "card_unlocked_dragon",
    "card_unlocked_exploding_deer",
    "card_unlocked_firework",
    "card_unlocked_maths",
    "card_unlocked_paint",
    "card_unlocked_pyramid",
    "card_unlocked_spiral_shot",
];

const REPORTED_FULL_FLAGS: &[&str] = &[
    "card_unlocked_black_hole",
    "card_unlocked_cessation",
    "card_unlocked_cloud_thunder",
    "card_unlocked_crumbling_earth",
    "card_unlocked_destruction",
    "card_unlocked_dragon",
    "card_unlocked_duplicate",
    "card_unlocked_everything",
    "card_unlocked_exploding_deer",
    "card_unlocked_kantele",
    "card_unlocked_maggot",
    "card_unlocked_material_cement",
    "card_unlocked_maths",
    "card_unlocked_mestari",
    "card_unlocked_musicbox",
    "card_unlocked_nuke",
    "card_unlocked_ocarina",
    "card_unlocked_paint",
    "card_unlocked_piss",
    "card_unlocked_pyramid",
    "card_unlocked_rain",
    "card_unlocked_sea_mimic",
    "card_unlocked_spiral_shot",
    "card_unlocked_tentacle",
    "card_unlocked_touch_grass",
    "perk_picked_no_more_shuffle",
];

fn deck_ids(spells: &[Spell]) -> Vec<&'static str> {
    spells.iter().map(|spell| spell.id()).collect()
}

fn reported_unlock_flags() -> Vec<String> {
    REPORTED_UNLOCK_FLAGS
        .iter()
        .map(|flag| (*flag).to_string())
        .collect()
}

fn great_chest_wands(x: i32, y: i32, seed: u32, unlock_flags: &[String]) -> Vec<Wand> {
    LootSpawner::new(
        seed,
        &GREAT_CHEST_LOOT_TABLE,
        Some(SaveFlags::new(unlock_flags.to_vec())),
    )
    .spawn(SpawnCoord { x, y })
    .into_iter()
    .filter_map(|item| match item {
        Item::Wand(wand) => Some(wand),
        _ => None,
    })
    .collect()
}

#[test]
fn eoe_wand_deck_matches_reported_chest_wand() {
    let unlock_flags = reported_unlock_flags();
    let wand = great_chest_wands(-7, -156, 322_255_393, &unlock_flags)
        .into_iter()
        .next()
        .expect("reported location should contain a wand");

    assert_eq!(wand.always_cast, Spell::None);
    assert_eq!(
        deck_ids(&wand.spells),
        vec![
            "NOLLA",
            "RECHARGE",
            "CIRCLE_SHAPE",
            "FIZZLE",
            "EXPLODING_DUCKS",
            "EXPLODING_DUCKS",
            "HEAVY_SPREAD",
            "HITFX_EXPLOSION_ALCOHOL",
            "RECHARGE",
            "AREA_DAMAGE",
            "MANA_REDUCE",
            "LASER_EMITTER_WIDER",
            "NONE",
            "NONE",
            "NONE",
        ]
    );
}

#[test]
fn eoe_level_10_chest_wand_uses_level_10_generator() {
    let unlock_flags = Vec::new();
    let wand = great_chest_wands(-2467, -7654, 322_255_393, &unlock_flags)
        .into_iter()
        .next()
        .expect("reported high-tier EoE chest location should contain a wand");

    assert_eq!(wand.capacity, 64);
    assert_eq!(wand.multicast, 7);
    assert_eq!(wand.mana, 1670);
    assert_eq!(wand.regen, 591);
    assert_eq!(wand.delay, 5.0 / 60.0);
    assert_eq!(wand.reload, 139.0 / 60.0);
    assert_eq!(wand.spread, 34);
}

#[test]
fn eoe_reported_full_flags_keep_final_ground_to_sand_and_sprite() {
    let unlock_flags = REPORTED_FULL_FLAGS
        .iter()
        .map(|flag| (*flag).to_string())
        .collect::<Vec<_>>();
    let save_flags = SaveFlags::new(unlock_flags.clone());
    let coord = SpawnCoord { x: -2467, y: -7654 };
    let wand = great_chest_wands(coord.x, coord.y, 322_255_393, &unlock_flags)
        .into_iter()
        .next()
        .expect("reported high-tier EoE chest location should contain a wand");
    let ids = deck_ids(&wand.spells);

    assert_eq!(
        &ids[38..41],
        &["RANDOM_MODIFIER", "ORBIT_NUKES", "STATIC_TO_SAND"]
    );
    assert!(!wand.shuffle);
    assert_eq!(
        find_wand_sprite(
            322_255_393,
            &GREAT_CHEST_LOOT_TABLE,
            Some(&save_flags),
            coord,
            &wand,
        ),
        Some(260),
    );
}

#[test]
fn slow_mana_branch_uses_lua_random_offset_range() {
    let wand = get_wand_unlocked(1, -20.0, -7.0, 60, 3, false, None);

    assert_eq!(wand.mana, 1530);
}

fn request_with_unlocks(unlock_flags: Vec<String>) -> SearchRequest {
    let wand_filters = WandFilterSet {
        filters: vec![WandFilter::spell_deck_requirement(Spell::Nolla, 1)],
    };
    SearchRequest {
        seed: 322_255_393,
        ng: 0,
        start_x: -7.0,
        start_y: -156.0,
        mode: SearchMode::EoeWand,
        wand_filters,
        unlock_flags: Some(unlock_flags),
    }
}

#[test]
fn search_request_unlock_flags_change_generated_deck() {
    let reported_flags = REPORTED_UNLOCK_FLAGS
        .iter()
        .map(|flag| (*flag).to_string())
        .collect::<Vec<_>>();
    let mut unlocked_state = SearchState::new(request_with_unlocks(reported_flags));
    assert!(matches!(
        unlocked_state.step(1),
        Some(SearchHit::Wand { .. })
    ));

    let mut locked_state = SearchState::new(request_with_unlocks(Vec::new()));
    assert!(locked_state.step(1).is_none());
}

fn first_search_wand(mode: SearchMode, start_x: f64, start_y: f64) -> Wand {
    let mut state = SearchState::new(SearchRequest {
        seed: 123_456,
        ng: 0,
        start_x,
        start_y,
        mode,
        wand_filters: WandFilterSet {
            filters: Vec::new(),
        },
        unlock_flags: None,
    });
    match state.step(1) {
        Some(SearchHit::Wand { wand, .. }) => wand,
        None => panic!("unfiltered one-step wand search should return the starting wand"),
    }
}

#[test]
fn taikasauva_search_uses_loot_table_wand_generation() {
    let wand = first_search_wand(SearchMode::TaikasauvaWand, 10.0, 20.0);
    let expected = get_wand_unlocked(123_456, 10.0, 20.0, 60, 3, false, None);
    assert_eq!(wand, expected);
}

#[test]
fn tiny_drop_search_uses_loot_table_wand_generation() {
    let wand = first_search_wand(SearchMode::TinyDropWand, 10.0, 20.0);
    let expected = get_wand_unlocked(123_456, 26.0, 20.0, 180, 11, true, None);
    assert_eq!(wand, expected);
}
