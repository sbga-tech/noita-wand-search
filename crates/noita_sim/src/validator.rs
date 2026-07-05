use crate::data::{ActionType, Spell, SPELL_PROBS_TYPES};
use crate::filters::{Comparison, FilterMode, WandFilter, WandFilterKind, WandFilterSet};
use crate::loot::great_chest_wand_generator_weights;
use crate::search::{SearchMode, SearchRequest};
use crate::types::WandStat;
use std::collections::HashSet;
use std::fmt;

pub const ASTRONOMICAL_SEARCH_THRESHOLD: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct FilterCheck {
    pub label: String,
    pub estimated_chance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidationReport {
    pub filters: Vec<FilterCheck>,
    pub estimated_match_chance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    Impossible(String),
    SearchTooRare {
        estimated_match_chance: f64,
        threshold: f64,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Impossible(message) => f.write_str(message),
            Self::SearchTooRare {
                estimated_match_chance,
                threshold,
            } => write!(
                f,
                "Filter combination is too rare to search: estimated match chance {estimated_match_chance:.3e} is below {threshold:.1e}. Loosen at least one predicate before searching."
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConditionRelation {
    Never,
    Sometimes,
    Always,
}

#[derive(Clone, Copy)]
struct NumericRange {
    min: f64,
    max: f64,
    quantum: Option<f64>,
}

impl NumericRange {
    const fn continuous(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            quantum: None,
        }
    }

    const fn stepped(min: f64, max: f64, quantum: f64) -> Self {
        Self {
            min,
            max,
            quantum: Some(quantum),
        }
    }

    fn relation(self, comparison: &Comparison, value: f64) -> ConditionRelation {
        match comparison {
            Comparison::Equals => {
                if value < self.min || value > self.max {
                    ConditionRelation::Never
                } else if self.min == self.max && value == self.min {
                    ConditionRelation::Always
                } else {
                    ConditionRelation::Sometimes
                }
            }
            Comparison::NotEquals => {
                if value < self.min || value > self.max {
                    ConditionRelation::Always
                } else if self.min == self.max && value == self.min {
                    ConditionRelation::Never
                } else {
                    ConditionRelation::Sometimes
                }
            }
            Comparison::LessThan => {
                if self.min >= value {
                    ConditionRelation::Never
                } else if self.max < value {
                    ConditionRelation::Always
                } else {
                    ConditionRelation::Sometimes
                }
            }
            Comparison::LessThanOrEquals => {
                if self.min > value {
                    ConditionRelation::Never
                } else if self.max <= value {
                    ConditionRelation::Always
                } else {
                    ConditionRelation::Sometimes
                }
            }
            Comparison::GreaterThan => {
                if self.max <= value {
                    ConditionRelation::Never
                } else if self.min > value {
                    ConditionRelation::Always
                } else {
                    ConditionRelation::Sometimes
                }
            }
            Comparison::GreaterThanOrEquals => {
                if self.max < value {
                    ConditionRelation::Never
                } else if self.min >= value {
                    ConditionRelation::Always
                } else {
                    ConditionRelation::Sometimes
                }
            }
        }
    }

    fn estimate(self, comparison: &Comparison, value: f64) -> f64 {
        if let Some(quantum) = self.quantum {
            return self.stepped_estimate(comparison, value, quantum);
        }
        let width = (self.max - self.min).max(f64::EPSILON);
        match comparison {
            Comparison::Equals => {
                if value < self.min || value > self.max {
                    0.0
                } else {
                    1.0e-9
                }
            }
            Comparison::NotEquals => {
                if value < self.min || value > self.max {
                    1.0
                } else {
                    1.0 - 1.0e-9
                }
            }
            Comparison::LessThan => ((value - self.min) / width).clamp(0.0, 1.0),
            Comparison::LessThanOrEquals => ((value - self.min) / width).clamp(0.0, 1.0),
            Comparison::GreaterThan => ((self.max - value) / width).clamp(0.0, 1.0),
            Comparison::GreaterThanOrEquals => ((self.max - value) / width).clamp(0.0, 1.0),
        }
    }

    fn stepped_estimate(self, comparison: &Comparison, value: f64, quantum: f64) -> f64 {
        let steps = ((self.max - self.min) / quantum).round() as i64 + 1;
        let matching = (0..steps)
            .filter(|step| {
                let candidate = self.min + *step as f64 * quantum;
                match comparison {
                    Comparison::Equals => (candidate - value).abs() <= quantum / 1000.0,
                    Comparison::NotEquals => (candidate - value).abs() > quantum / 1000.0,
                    Comparison::LessThan => candidate < value,
                    Comparison::LessThanOrEquals => candidate <= value,
                    Comparison::GreaterThan => candidate > value,
                    Comparison::GreaterThanOrEquals => candidate >= value,
                }
            })
            .count() as f64;
        matching / steps as f64
    }
}

fn mode_stat_range(mode: SearchMode, stat: WandStat) -> NumericRange {
    match stat {
        WandStat::Capacity => NumericRange::stepped(2.0, max_capacity(mode) as f64, 1.0),
        WandStat::Multicast => NumericRange::stepped(1.0, max_capacity(mode) as f64, 1.0),
        WandStat::CastDelay => NumericRange::stepped(-15.0 / 60.0, 50.0 / 60.0, 1.0 / 60.0),
        WandStat::Reload => NumericRange::stepped(1.0 / 60.0, 240.0 / 60.0, 1.0 / 60.0),
        WandStat::MaxMana => match mode {
            SearchMode::TaikasauvaWand => NumericRange::stepped(150.0, 1650.0, 1.0),
            SearchMode::TinyDropWand => NumericRange::stepped(550.0, 5250.0, 1.0),
            SearchMode::EoeWand => NumericRange::stepped(200.0, 5250.0, 1.0),
        },
        WandStat::ManaRegen => match mode {
            SearchMode::TaikasauvaWand => NumericRange::stepped(29.0, 825.0, 1.0),
            SearchMode::TinyDropWand => NumericRange::stepped(109.0, 3025.0, 1.0),
            SearchMode::EoeWand => NumericRange::stepped(39.0, 3025.0, 1.0),
        },
        WandStat::Spread => NumericRange::stepped(-35.0, 35.0, 1.0),
        WandStat::Speed => NumericRange::continuous(0.5, 10.0),
    }
}

fn max_capacity(mode: SearchMode) -> i32 {
    match mode {
        SearchMode::TaikasauvaWand => 49,
        SearchMode::TinyDropWand => 73,
        SearchMode::EoeWand => 77,
    }
}

fn mode_can_shuffle(mode: SearchMode) -> (bool, bool) {
    match mode {
        SearchMode::TinyDropWand => (false, true),
        SearchMode::EoeWand | SearchMode::TaikasauvaWand => (true, true),
    }
}

fn for_each_wand_spell_level(mode: SearchMode, mut visit: impl FnMut(i32)) {
    match mode {
        SearchMode::EoeWand => {
            for (generator, _) in great_chest_wand_generator_weights() {
                visit(generator.spell_level());
            }
        }
        SearchMode::TaikasauvaWand => visit(3),
        SearchMode::TinyDropWand => visit(11),
    }
}

fn table_level(level: i32) -> usize {
    level.clamp(0, 10) as usize
}

fn insert_type_spells(
    out: &mut HashSet<Spell>,
    level: usize,
    action_type: ActionType,
    request: &SearchRequest,
) {
    let type_index = usize::from(u8::from(action_type));
    for prob in SPELL_PROBS_TYPES[level][type_index] {
        if spell_unlocked_for_request(prob.spell, request) {
            out.insert(prob.spell);
        }
    }
}

fn possible_deck_spells(request: &SearchRequest) -> HashSet<Spell> {
    let mut spells = HashSet::new();
    for_each_wand_spell_level(request.mode, |raw_level| {
        let level = table_level(raw_level - 1);
        insert_type_spells(&mut spells, level, ActionType::Projectile, request);
        insert_type_spells(&mut spells, level, ActionType::Modifier, request);
        insert_type_spells(&mut spells, level, ActionType::DrawMany, request);
    });
    spells.insert(Spell::None);
    spells
}

fn possible_always_cast_spells(request: &SearchRequest) -> HashSet<Spell> {
    let mut spells = HashSet::new();
    spells.insert(Spell::None);
    for_each_wand_spell_level(request.mode, |raw_level| {
        let level = table_level(raw_level);
        insert_type_spells(&mut spells, level, ActionType::Projectile, request);
        insert_type_spells(&mut spells, level, ActionType::StaticProjectile, request);
        insert_type_spells(&mut spells, level, ActionType::Modifier, request);
    });
    spells
}

fn spell_unlocked_for_request(spell: Spell, request: &SearchRequest) -> bool {
    let Some(flag) = spell.unlock_flag() else {
        return true;
    };
    match request.unlock_flags.as_deref() {
        Some(flags) => flags.iter().any(|candidate| candidate == flag),
        None => true,
    }
}

fn validate_spell(
    label: &str,
    filter: &WandFilter,
    possible_spells: &HashSet<Spell>,
    spell: Spell,
) -> Result<FilterCheck, ValidationError> {
    let possible = possible_spells.contains(&spell);
    let relation = if possible {
        ConditionRelation::Sometimes
    } else {
        ConditionRelation::Never
    };
    reject_impossible(filter.mode, relation, || {
        format!(
            "{label} cannot contain {} with the current mode/unlock flags.",
            spell.display_name("en")
        )
    })?;
    let base = if possible {
        1.0 / possible_spells.len().max(1) as f64
    } else {
        0.0
    };
    Ok(FilterCheck {
        label: format!("{label}: {}", spell.display_name("en")),
        estimated_chance: apply_filter_mode_estimate(filter.mode, base),
    })
}

fn reject_impossible<F>(
    mode: FilterMode,
    relation: ConditionRelation,
    message: F,
) -> Result<(), ValidationError>
where
    F: FnOnce() -> String,
{
    match (mode, relation) {
        (FilterMode::Include, ConditionRelation::Never)
        | (FilterMode::Exclude, ConditionRelation::Always) => {
            Err(ValidationError::Impossible(message()))
        }
        _ => Ok(()),
    }
}

fn apply_filter_mode_estimate(mode: FilterMode, estimate: f64) -> f64 {
    match mode {
        FilterMode::Include => estimate,
        FilterMode::Exclude => 1.0 - estimate,
    }
}

fn validate_numeric_filter(
    mode: SearchMode,
    filter: &WandFilter,
    stat: WandStat,
    comparison: &Comparison,
    value: f64,
) -> Result<FilterCheck, ValidationError> {
    let range = mode_stat_range(mode, stat);
    let relation = range.relation(comparison, value);
    reject_impossible(filter.mode, relation, || {
        format!(
            "{} filter is outside the possible range ({:.4}..{:.4}).",
            stat.label(),
            range.min,
            range.max
        )
    })?;
    let base = range.estimate(comparison, value);
    Ok(FilterCheck {
        label: format!("{} {comparison:?} {value}", stat.label()),
        estimated_chance: apply_filter_mode_estimate(filter.mode, base),
    })
}

fn validate_shuffle_filter(
    mode: SearchMode,
    filter: &WandFilter,
    comparison: &Comparison,
    value: bool,
) -> Result<FilterCheck, ValidationError> {
    let (can_true, can_false) = mode_can_shuffle(mode);
    let matching_possible = match comparison {
        Comparison::Equals => (value && can_true) || (!value && can_false),
        Comparison::NotEquals => (value && can_false) || (!value && can_true),
        _ => false,
    };
    let relation = if matching_possible {
        ConditionRelation::Sometimes
    } else {
        ConditionRelation::Never
    };
    reject_impossible(filter.mode, relation, || {
        "shuffle filter is outside the possible range for this search mode.".to_string()
    })?;
    Ok(FilterCheck {
        label: format!("shuffle {comparison:?} {value}"),
        estimated_chance: apply_filter_mode_estimate(filter.mode, 0.5),
    })
}

fn validate_deck_filter(
    request: &SearchRequest,
    filter: &WandFilter,
    spell: Spell,
    amount: usize,
) -> Result<FilterCheck, ValidationError> {
    if amount > max_capacity(request.mode) as usize {
        return Err(ValidationError::Impossible(format!(
            "spell deck requires {amount} copies but capacity cannot exceed {} in this mode.",
            max_capacity(request.mode)
        )));
    }
    validate_spell("spell deck", filter, &possible_deck_spells(request), spell)
}

fn validate_always_cast_filter(
    request: &SearchRequest,
    filter: &WandFilter,
    spell: Spell,
) -> Result<FilterCheck, ValidationError> {
    validate_spell(
        "always-cast",
        filter,
        &possible_always_cast_spells(request),
        spell,
    )
}

fn validate_filter(
    request: &SearchRequest,
    filter: &WandFilter,
) -> Result<FilterCheck, ValidationError> {
    match &filter.kind {
        WandFilterKind::Stat {
            stat,
            comparison,
            value,
        } => validate_numeric_filter(request.mode, filter, *stat, comparison, *value),
        WandFilterKind::Shuffle { comparison, value } => {
            validate_shuffle_filter(request.mode, filter, comparison, *value)
        }
        WandFilterKind::AlwaysCast { value, .. } => {
            validate_always_cast_filter(request, filter, *value)
        }
        WandFilterKind::SpellDeckRequirement { value, amount } => {
            validate_deck_filter(request, filter, *value, *amount)
        }
    }
}

pub fn validate_search_request(
    request: &SearchRequest,
) -> Result<ValidationReport, ValidationError> {
    let filters = request
        .wand_filters
        .filters
        .iter()
        .map(|filter| validate_filter(request, filter))
        .collect::<Result<Vec<_>, _>>()?;
    let estimated_match_chance = filters
        .iter()
        .map(|filter| filter.estimated_chance)
        .product::<f64>();
    if estimated_match_chance > 0.0
        && estimated_match_chance < ASTRONOMICAL_SEARCH_THRESHOLD
        && !filters.is_empty()
    {
        return Err(ValidationError::SearchTooRare {
            estimated_match_chance,
            threshold: ASTRONOMICAL_SEARCH_THRESHOLD,
        });
    }
    Ok(ValidationReport {
        filters,
        estimated_match_chance,
    })
}

pub fn validate_filter_set(
    mode: SearchMode,
    filters: WandFilterSet,
) -> Result<ValidationReport, ValidationError> {
    let request = SearchRequest {
        seed: 0,
        ng: 0,
        start_x: 0.0,
        start_y: 0.0,
        mode,
        wand_filters: filters,
        unlock_flags: None,
    };
    validate_search_request(&request)
}

trait WandStatLabel {
    fn label(self) -> &'static str;
}

impl WandStatLabel for WandStat {
    fn label(self) -> &'static str {
        match self {
            WandStat::Capacity => "capacity",
            WandStat::Multicast => "multicast",
            WandStat::CastDelay => "cast delay",
            WandStat::Reload => "reload",
            WandStat::MaxMana => "max mana",
            WandStat::ManaRegen => "mana regen",
            WandStat::Spread => "spread",
            WandStat::Speed => "speed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{Comparison, WandFilter, WandFilterKind};

    fn request_with_filter(filter: WandFilter) -> SearchRequest {
        SearchRequest {
            seed: 1,
            ng: 0,
            start_x: 0.0,
            start_y: 0.0,
            mode: SearchMode::EoeWand,
            wand_filters: WandFilterSet {
                filters: vec![filter],
            },
            unlock_flags: None,
        }
    }

    #[test]
    fn rejects_out_of_range_stat_filters() {
        let request = request_with_filter(WandFilter::stat(
            WandStat::Capacity,
            Comparison::GreaterThan,
            77.0,
        ));
        assert!(matches!(
            validate_search_request(&request),
            Err(ValidationError::Impossible(_))
        ));
    }

    #[test]
    fn rejects_static_projectiles_in_eoe_deck() {
        let request =
            request_with_filter(WandFilter::spell_deck_requirement(Spell::FreezeField, 1));
        assert!(matches!(
            validate_search_request(&request),
            Err(ValidationError::Impossible(_))
        ));
    }

    #[test]
    fn accepts_static_projectiles_as_eoe_always_cast() {
        let request = request_with_filter(WandFilter::always_cast(
            Comparison::Equals,
            Spell::FreezeField,
        ));
        assert!(validate_search_request(&request).is_ok());
    }

    #[test]
    fn rejects_tiny_drop_shuffle_true() {
        let mut request = request_with_filter(WandFilter::shuffle(Comparison::Equals, true));
        request.mode = SearchMode::TinyDropWand;
        assert!(matches!(
            validate_search_request(&request),
            Err(ValidationError::Impossible(_))
        ));
    }

    #[test]
    fn rejects_deck_amount_above_capacity() {
        let request =
            request_with_filter(WandFilter::include(WandFilterKind::SpellDeckRequirement {
                value: Spell::Nolla,
                amount: 78,
            }));
        assert!(matches!(
            validate_search_request(&request),
            Err(ValidationError::Impossible(_))
        ));
    }
}
