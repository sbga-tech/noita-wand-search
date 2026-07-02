use gloo_timers::future::TimeoutFuture;
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal};
use leptos::task::spawn_local;
use noita_sim::search::{SearchHit, SearchMode, SearchProgress, SearchRequest, SearchState};

pub fn batch_size(mode: SearchMode) -> u32 {
    if mode == SearchMode::EoeWand {
        38_373
    } else {
        19_287
    }
}

pub fn run_search_until_hit(request: SearchRequest, max_batches: u32) -> (u32, Option<SearchHit>) {
    let mut state = SearchState::new(request.clone());
    for batch in 0..max_batches {
        if let Some(hit) = state.step(batch_size(request.mode)) {
            return (batch, Some(hit));
        }
    }
    (max_batches, None)
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_client_search(
    request: SearchRequest,
    token_id: u64,
    active_token: ReadSignal<u64>,
    status: WriteSignal<String>,
    result: WriteSignal<Option<SearchHit>>,
    progress: WriteSignal<SearchProgress>,
    searching: WriteSignal<bool>,
) {
    spawn_local(async move {
        let mut state = SearchState::new(request.clone());
        result.set(None);
        status.set("Searching...".to_string());
        loop {
            if active_token.get_untracked() != token_id {
                return;
            }
            if let Some(hit) = state.step(batch_size(request.mode)) {
                let current = state.progress();
                status.set(format!("{} pixels checked...", current.searched_pixels));
                progress.set(current);
                result.set(Some(hit));
                searching.set(false);
                return;
            }
            let current = state.progress();
            status.set(format!("{} pixels checked...", current.searched_pixels));
            progress.set(current);
            TimeoutFuture::new(16).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::control_panel::{
        apply_mode_side_effects, default_form_state, validate_state, PredicateAttribute,
        PredicateInput, PredicateValue,
    };
    use noita_sim::filters::{Comparison, WandFilterKind};
    use noita_sim::WandStat;

    #[test]
    fn default_query_matches_current_defaults() {
        let state = default_form_state();
        let request = validate_state(&state).unwrap();
        assert_eq!(request.seed, 1);
        assert_eq!(request.ng, 0);
        assert_eq!(request.mode, SearchMode::EoeWand);
        assert_eq!(request.wand_filters.filters.len(), 1);
        match &request.wand_filters.filters[0].kind {
            WandFilterKind::Stat {
                stat,
                comparison,
                value,
            } => {
                assert_eq!(*stat, WandStat::Capacity);
                assert_eq!(*comparison, Comparison::GreaterThanOrEquals);
                assert_eq!(*value, 26.0);
            }
            other => panic!("expected stat filter, got {other:?}"),
        }
        let unlock_flags = request.unlock_flags.as_ref().unwrap();
        assert!(unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_pyramid"));
        assert!(unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_exploding_deer"));
        assert_eq!(unlock_flags.len(), 38);
    }

    #[test]
    fn validation_preserves_empty_unlock_selection() {
        let mut state = default_form_state();
        state.unlock_flags.clear();
        let request = validate_state(&state).unwrap();
        assert_eq!(request.unlock_flags, Some(Vec::new()));
    }

    #[test]
    fn validation_preserves_empty_predicate_selection() {
        let mut state = default_form_state();
        state.predicates.clear();
        let request = validate_state(&state).unwrap();
        assert!(request.wand_filters.filters.is_empty());
    }

    #[test]
    fn integer_predicates_floor_float_input_before_comparison() {
        let mut state = default_form_state();
        state.predicates[0].value = PredicateValue::Number("25.9".into());
        let request = validate_state(&state).unwrap();
        match &request.wand_filters.filters[0].kind {
            WandFilterKind::Stat { value, .. } => assert_eq!(*value, 25.0),
            other => panic!("expected stat filter, got {other:?}"),
        }
    }
    #[test]
    fn tiny_drop_selection_sets_coordinates() {
        let mut state = default_form_state();
        apply_mode_side_effects(&mut state, SearchMode::TinyDropWand);
        assert_eq!(state.x, "14941");
        assert_eq!(state.y, "18654");
    }

    #[test]
    fn validation_messages_match_contract() {
        let mut state = default_form_state();
        state.predicates[0].value = PredicateValue::Number(String::new());
        assert_eq!(
            validate_state(&state).unwrap_err(),
            "Invalid number in Value."
        );
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput::new(2, PredicateAttribute::AlwaysCast)];
        state.predicates[0].value = PredicateValue::String("not a spell".into());
        assert_eq!(validate_state(&state).unwrap_err(), "Invalid spell!");

        let defaults = [
            (
                PredicateAttribute::Capacity,
                Comparison::GreaterThanOrEquals,
                PredicateValue::Number("26".into()),
            ),
            (
                PredicateAttribute::Multicast,
                Comparison::Equals,
                PredicateValue::Number("1".into()),
            ),
            (
                PredicateAttribute::CastDelay,
                Comparison::LessThanOrEquals,
                PredicateValue::Number("0".into()),
            ),
            (
                PredicateAttribute::Reload,
                Comparison::LessThan,
                PredicateValue::Number("0.05".into()),
            ),
            (
                PredicateAttribute::MaxMana,
                Comparison::GreaterThan,
                PredicateValue::Number("500".into()),
            ),
            (
                PredicateAttribute::ManaRegen,
                Comparison::GreaterThan,
                PredicateValue::Number("1000".into()),
            ),
            (
                PredicateAttribute::Spread,
                Comparison::Equals,
                PredicateValue::Number("0".into()),
            ),
            (
                PredicateAttribute::Speed,
                Comparison::GreaterThanOrEquals,
                PredicateValue::Number("1".into()),
            ),
            (
                PredicateAttribute::Shuffle,
                Comparison::Equals,
                PredicateValue::Boolean(false),
            ),
            (
                PredicateAttribute::SpellDeck,
                Comparison::Equals,
                PredicateValue::String("Add mana".into()),
            ),
            (
                PredicateAttribute::AlwaysCast,
                Comparison::Equals,
                PredicateValue::String("Add mana".into()),
            ),
        ];
        for (attribute, comparison, value) in defaults {
            let input = PredicateInput::new(1, attribute);
            assert_eq!(input.comparison, comparison);
            assert_eq!(input.value, value);
        }
    }
}
