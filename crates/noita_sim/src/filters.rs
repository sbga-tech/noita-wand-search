use crate::data::Spell;
use crate::types::{Wand, WandStat};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Comparison {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterMode {
    #[default]
    Include,
    Exclude,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WandFilterKind {
    Stat {
        stat: WandStat,
        comparison: Comparison,
        value: f64,
    },
    Shuffle {
        comparison: Comparison,
        value: bool,
    },
    AlwaysCast {
        comparison: Comparison,
        value: Spell,
    },
    SpellDeckRequirement {
        value: Spell,
        amount: usize,
    },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WandFilter {
    #[serde(default)]
    pub mode: FilterMode,
    pub kind: WandFilterKind,
}

impl WandFilter {
    pub fn include(kind: WandFilterKind) -> Self {
        Self {
            mode: FilterMode::Include,
            kind,
        }
    }

    pub fn exclude(kind: WandFilterKind) -> Self {
        Self {
            mode: FilterMode::Exclude,
            kind,
        }
    }

    pub fn stat(stat: WandStat, comparison: Comparison, value: f64) -> Self {
        Self::include(WandFilterKind::Stat {
            stat,
            comparison,
            value,
        })
    }

    pub fn shuffle(comparison: Comparison, value: bool) -> Self {
        Self::include(WandFilterKind::Shuffle { comparison, value })
    }

    pub fn always_cast(comparison: Comparison, value: Spell) -> Self {
        Self::include(WandFilterKind::AlwaysCast { comparison, value })
    }

    pub fn spell_deck_requirement(value: Spell, amount: usize) -> Self {
        Self::include(WandFilterKind::SpellDeckRequirement { value, amount })
    }

    pub fn with_mode(mut self, mode: FilterMode) -> Self {
        self.mode = mode;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WandFilterSet {
    pub filters: Vec<WandFilter>,
}

pub fn stat_value(wand: &Wand, stat: WandStat) -> f64 {
    match stat {
        WandStat::Capacity => wand.capacity as f64,
        WandStat::Multicast => wand.multicast as f64,
        WandStat::CastDelay => wand.delay,
        WandStat::Reload => wand.reload,
        WandStat::MaxMana => wand.mana as f64,
        WandStat::ManaRegen => wand.regen as f64,
        WandStat::Spread => wand.spread as f64,
        WandStat::Speed => wand.speed as f64,
    }
}

fn comparison_matches_f64(value: f64, comparison: &Comparison, expected: f64) -> bool {
    match comparison {
        Comparison::Equals => (value - expected).abs() < f64::EPSILON,
        Comparison::NotEquals => (value - expected).abs() >= f64::EPSILON,
        Comparison::LessThan => value < expected,
        Comparison::LessThanOrEquals => value <= expected,
        Comparison::GreaterThan => value > expected,
        Comparison::GreaterThanOrEquals => value >= expected,
    }
}

fn comparison_matches_bool(value: bool, comparison: &Comparison, expected: bool) -> bool {
    match comparison {
        Comparison::Equals => value == expected,
        Comparison::NotEquals => value != expected,
        _ => false,
    }
}

fn comparison_matches_spell(value: Spell, comparison: &Comparison, expected: Spell) -> bool {
    match comparison {
        Comparison::Equals => value == expected,
        Comparison::NotEquals => value != expected,
        _ => false,
    }
}

fn deck_contains_amount(wand: &Wand, value: Spell, amount: usize) -> bool {
    wand.spells.iter().filter(|spell| **spell == value).count() >= amount
}

fn filter_matches(wand: &Wand, filter: &WandFilter) -> bool {
    let matched = match &filter.kind {
        WandFilterKind::Stat {
            stat,
            comparison,
            value,
        } => comparison_matches_f64(stat_value(wand, *stat), comparison, *value),
        WandFilterKind::Shuffle { comparison, value } => {
            comparison_matches_bool(wand.shuffle, comparison, *value)
        }
        WandFilterKind::AlwaysCast { comparison, value } => {
            comparison_matches_spell(wand.always_cast, comparison, *value)
        }
        WandFilterKind::SpellDeckRequirement { value, amount } => {
            deck_contains_amount(wand, *value, *amount)
        }
    };
    match filter.mode {
        FilterMode::Include => matched,
        FilterMode::Exclude => !matched,
    }
}

pub fn wand_matches_filters(wand: &Wand, filters: &WandFilterSet) -> bool {
    filters
        .filters
        .iter()
        .all(|filter| filter_matches(wand, filter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_stat_uses_public_integer_value() {
        let wand = Wand {
            capacity: 25,
            ..Wand::default()
        };
        assert_eq!(stat_value(&wand, WandStat::Capacity), 25.0);
    }
}
