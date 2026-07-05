use crate::search_worker_protocol::{SearchWorkerEvent, SearchWorkerStart};
use js_sys::Uint8Array;
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal};
use noita_sim::search::{SearchHit, SearchProgress, SearchRequest};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType};

struct ActiveSearchWorker {
    worker: Worker,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
}

impl Drop for ActiveSearchWorker {
    fn drop(&mut self) {
        self.worker.terminate();
    }
}

thread_local! {
    static ACTIVE_SEARCH_WORKER: RefCell<Option<ActiveSearchWorker>> = const { RefCell::new(None) };
}

pub fn compact_number(value: f64) -> String {
    const UNITS: &[(f64, &str)] = &[
        (1_000_000_000_000.0, "T"),
        (1_000_000_000.0, "B"),
        (1_000_000.0, "M"),
        (1_000.0, "K"),
    ];
    let magnitude = value.abs();
    for (threshold, suffix) in UNITS {
        if magnitude >= *threshold {
            return format!("{:.3}{suffix}", value / threshold);
        }
    }
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
    }
}

fn format_search_status(searched_pixels: u64, pixels_per_second: f64) -> String {
    format!(
        "{} px {} px/s",
        compact_number(searched_pixels as f64),
        compact_number(pixels_per_second)
    )
}

pub fn cancel_active_search_worker() {
    ACTIVE_SEARCH_WORKER.with(|active| {
        active.borrow_mut().take();
    });
}

fn clear_active_search_worker(token_id: u64, active_token: ReadSignal<u64>) {
    if active_token.get_untracked() == token_id {
        cancel_active_search_worker();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_client_search(
    request: SearchRequest,
    token_id: u64,
    active_token: ReadSignal<u64>,
    status: WriteSignal<String>,
    result: WriteSignal<Option<SearchHit>>,
    progress: WriteSignal<SearchProgress>,
    search_speed: WriteSignal<f64>,
    searching: WriteSignal<bool>,
) {
    cancel_active_search_worker();

    let start_message = match bincode::serialize(&SearchWorkerStart { token_id, request }) {
        Ok(message) => message,
        Err(error) => {
            status.set(format!("Failed to encode search request: {error}"));
            search_speed.set(0.0);
            searching.set(false);
            return;
        }
    };

    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = match Worker::new_with_options("search_worker_loader.js", &options) {
        Ok(worker) => worker,
        Err(error) => {
            status.set(format!("Failed to start search worker: {error:?}"));
            searching.set(false);
            search_speed.set(0.0);
            return;
        }
    };

    let worker_for_message = worker.clone();
    let on_message =
        Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
            if active_token.get_untracked() != token_id {
                return;
            }

            let message = Uint8Array::new(&event.data()).to_vec();

            match bincode::deserialize::<SearchWorkerEvent>(&message) {
                Ok(SearchWorkerEvent::Ready) => {
                    let message = Uint8Array::from(start_message.as_slice());
                    if let Err(error) = worker_for_message.post_message(&message) {
                        status.set(format!(
                            "Failed to post search request to worker: {error:?}"
                        ));
                        searching.set(false);
                        search_speed.set(0.0);
                        clear_active_search_worker(token_id, active_token);
                    }
                }
                Ok(SearchWorkerEvent::Progress {
                    token_id: event_token,
                    progress: current,
                    pixels_per_second,
                }) if event_token == token_id => {
                    status.set(format_search_status(
                        current.searched_pixels,
                        pixels_per_second,
                    ));
                    search_speed.set(pixels_per_second);
                    progress.set(current);
                }
                Ok(SearchWorkerEvent::Hit {
                    token_id: event_token,
                    progress: current,
                    pixels_per_second,
                    hit,
                }) if event_token == token_id => {
                    status.set(format_search_status(
                        current.searched_pixels,
                        pixels_per_second,
                    ));
                    search_speed.set(pixels_per_second);
                    progress.set(current);
                    result.set(Some(hit));
                    searching.set(false);
                    clear_active_search_worker(token_id, active_token);
                }
                Ok(SearchWorkerEvent::Error {
                    token_id: event_token,
                    message,
                }) if event_token.is_none() || event_token == Some(token_id) => {
                    status.set(format!("Search worker error: {message}"));
                    searching.set(false);
                    search_speed.set(0.0);
                    clear_active_search_worker(token_id, active_token);
                }
                Ok(_) => {}
                Err(error) => {
                    status.set(format!("Failed to decode search worker message: {error}"));
                    searching.set(false);
                    clear_active_search_worker(token_id, active_token);
                }
            }
        }));

    let on_error = Closure::<dyn FnMut(ErrorEvent)>::wrap(Box::new(move |event: ErrorEvent| {
        if active_token.get_untracked() == token_id {
            status.set(format!("Search worker error: {}", event.message()));
            searching.set(false);
            clear_active_search_worker(token_id, active_token);
        }
    }));

    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    ACTIVE_SEARCH_WORKER.with(|active| {
        *active.borrow_mut() = Some(ActiveSearchWorker {
            worker,
            _on_message: on_message,
            _on_error: on_error,
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::control_panel::{
        apply_mode_side_effects, default_form_state, validate_state, PredicateAttribute,
        PredicateInput, PredicateValue,
    };
    use crate::components::unlock_settings::all_unlock_flags;
    use noita_sim::filters::{Comparison, WandFilterKind};
    use noita_sim::search::SearchMode;
    use noita_sim::WandStat;

    #[test]
    fn search_status_compacts_pixels_and_speed() {
        assert_eq!(
            format_search_status(12_345, 6_172.5),
            "12.345K px 6.173K px/s"
        );
    }

    #[test]
    fn compact_number_uses_metric_suffixes() {
        assert_eq!(compact_number(999.0), "999");
        assert_eq!(compact_number(1_000.0), "1.000K");
        assert_eq!(compact_number(1_234_567.0), "1.235M");
        assert_eq!(compact_number(1_234_567_890.0), "1.235B");
        assert_eq!(compact_number(1_234_567_890_123.0), "1.235T");
    }
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
        assert_eq!(unlock_flags, &all_unlock_flags());
        assert!(unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_pyramid"));
        assert!(unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_exploding_deer"));
        assert!(unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_funky"));
        assert!(!unlock_flags
            .iter()
            .any(|flag| flag == "card_unlocked_infinite"));
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
