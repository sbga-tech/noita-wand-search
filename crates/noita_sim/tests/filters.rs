use noita_sim::filters::{
    wand_matches_filters, Comparison, FilterMode, WandFilter, WandFilterKind, WandFilterSet,
};
use noita_sim::types::Wand;
use noita_sim::{Spell, WandStat};

fn add_mana() -> Spell {
    Spell::ManaReduce
}

fn set(filters: Vec<WandFilter>) -> WandFilterSet {
    WandFilterSet { filters }
}

fn sample_wand() -> Wand {
    Wand {
        capacity: 10,
        mana: 500,
        regen: 120,
        shuffle: false,
        always_cast: add_mana(),
        spells: [Spell::Bomb, Spell::LightBullet, Spell::Bomb]
            .into_iter()
            .collect(),
        ..Wand::default()
    }
}

#[test]
fn spell_names_are_locale_specific_and_exact() {
    assert_eq!(Spell::ManaReduce.display_name("en"), "Add mana");
    assert_eq!(Spell::ManaReduce.display_name("jp"), "マナを追加する");
    assert_eq!(
        Spell::from_display_name("Add mana", "en"),
        Some(Spell::ManaReduce)
    );
    assert_eq!(Spell::from_display_name("add mana", "en"), None);
    assert_eq!(
        Spell::from_display_name("マナを追加する", "jp"),
        Some(Spell::ManaReduce)
    );
    assert_eq!(
        Spell::from_display_name("Add mana", "missing-locale"),
        Some(Spell::ManaReduce)
    );
}

#[test]
fn stat_filters_are_anded() {
    let wand = sample_wand();
    let filters = set(vec![
        WandFilter::stat(WandStat::Capacity, Comparison::GreaterThan, 9.0),
        WandFilter::stat(WandStat::ManaRegen, Comparison::GreaterThan, 121.0),
    ]);
    assert!(!wand_matches_filters(&wand, &filters));
}

#[test]
fn numeric_comparisons_apply() {
    let wand = sample_wand();
    let capacity = set(vec![WandFilter::stat(
        WandStat::Capacity,
        Comparison::Equals,
        10.0,
    )]);
    let less_than_or_equals = set(vec![WandFilter::stat(
        WandStat::ManaRegen,
        Comparison::LessThanOrEquals,
        120.0,
    )]);
    let greater_than_or_equals = set(vec![WandFilter::stat(
        WandStat::MaxMana,
        Comparison::GreaterThanOrEquals,
        500.0,
    )]);
    let not_equals = set(vec![WandFilter::stat(
        WandStat::ManaRegen,
        Comparison::NotEquals,
        120.0,
    )]);
    assert!(wand_matches_filters(&wand, &capacity));
    assert!(wand_matches_filters(&wand, &less_than_or_equals));
    assert!(wand_matches_filters(&wand, &greater_than_or_equals));
    assert!(!wand_matches_filters(&wand, &not_equals));
}

#[test]
fn nonshuffle_and_no_always_cast_requirements_apply() {
    let mut wand = sample_wand();
    let filters = set(vec![
        WandFilter::shuffle(Comparison::Equals, false),
        WandFilter::always_cast(Comparison::Equals, Spell::None),
    ]);
    assert!(!wand_matches_filters(&wand, &filters));
    wand.always_cast = Spell::None;
    assert!(wand_matches_filters(&wand, &filters));
    wand.shuffle = true;
    assert!(!wand_matches_filters(&wand, &filters));
}

#[test]
fn shuffle_always_cast_and_deck_predicates_apply() {
    let wand = sample_wand();
    let shuffle_false = set(vec![WandFilter::shuffle(Comparison::Equals, false)]);
    let deck_contains_two_bombs = set(vec![WandFilter::spell_deck_requirement(Spell::Bomb, 2)]);
    let deck_excludes_add_mana = set(vec![
        WandFilter::spell_deck_requirement(add_mana(), 1).with_mode(FilterMode::Exclude)
    ]);
    let always_cast_add_mana = set(vec![WandFilter::always_cast(
        Comparison::Equals,
        add_mana(),
    )]);
    assert!(wand_matches_filters(&wand, &shuffle_false));
    assert!(wand_matches_filters(&wand, &deck_contains_two_bombs));
    assert!(wand_matches_filters(&wand, &deck_excludes_add_mana));
    assert!(wand_matches_filters(&wand, &always_cast_add_mana));
}

#[test]
fn exclude_mode_inverts_predicate_result() {
    let wand = sample_wand();
    let exclude_capacity_10 = set(vec![WandFilter::stat(
        WandStat::Capacity,
        Comparison::Equals,
        10.0,
    )
    .with_mode(FilterMode::Exclude)]);
    let exclude_deck_bomb = set(vec![
        WandFilter::spell_deck_requirement(Spell::Bomb, 1).with_mode(FilterMode::Exclude)
    ]);
    let exclude_shuffle_false = set(vec![
        WandFilter::shuffle(Comparison::Equals, false).with_mode(FilterMode::Exclude)
    ]);
    assert!(!wand_matches_filters(&wand, &exclude_capacity_10));
    assert!(!wand_matches_filters(&wand, &exclude_deck_bomb));
    assert!(!wand_matches_filters(&wand, &exclude_shuffle_false));
}

#[test]
fn explicit_filter_kind_can_be_wrapped_once() {
    let wand = sample_wand();
    let filters = set(vec![WandFilter {
        mode: FilterMode::Include,
        kind: WandFilterKind::Stat {
            stat: WandStat::Capacity,
            comparison: Comparison::Equals,
            value: 10.0,
        },
    }]);
    assert!(wand_matches_filters(&wand, &filters));
}
