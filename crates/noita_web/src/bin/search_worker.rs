use js_sys::Uint8Array;
use noita_sim::search::SearchState;
use noita_web::search_worker_protocol::{SearchWorkerEvent, SearchWorkerStart};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

const SEARCH_PROGRESS_CANDIDATES: u32 = 1 << 18;
const SEARCH_CANDIDATES_PER_THREAD: u32 = 8_192;
const MIN_SEARCH_BATCH_CANDIDATES: u32 = 1 << 14;
const MAX_SEARCH_BATCH_CANDIDATES: u32 = 1 << 16;

fn hardware_concurrency() -> usize {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator")).ok();
    let concurrency = navigator
        .and_then(|navigator| {
            js_sys::Reflect::get(&navigator, &JsValue::from_str("hardwareConcurrency")).ok()
        })
        .and_then(|value| value.as_f64())
        .unwrap_or(1.0);

    concurrency.max(1.0) as usize
}

fn search_batch_candidates(thread_count: usize) -> u32 {
    (thread_count as u32 * SEARCH_CANDIDATES_PER_THREAD)
        .clamp(MIN_SEARCH_BATCH_CANDIDATES, MAX_SEARCH_BATCH_CANDIDATES)
}

fn pixels_per_second(searched_pixels: u32, started_at_ms: f64, now_ms: f64) -> f64 {
    let elapsed_secs = ((now_ms - started_at_ms) / 1_000.0).max(0.001);
    searched_pixels as f64 / elapsed_secs
}

fn post_event(scope: &DedicatedWorkerGlobalScope, event: SearchWorkerEvent) {
    match bincode::serialize(&event) {
        Ok(message) => {
            let bytes = Uint8Array::from(message.as_slice());
            let _ = scope.post_message(&bytes);
        }
        Err(error) => {
            let fallback = SearchWorkerEvent::Error {
                token_id: None,
                message: format!("failed to serialize worker event: {error}"),
            };
            if let Ok(message) = bincode::serialize(&fallback) {
                let bytes = Uint8Array::from(message.as_slice());
                let _ = scope.post_message(&bytes);
            }
        }
    }
}

fn run_search(scope: &DedicatedWorkerGlobalScope, start: SearchWorkerStart, thread_count: usize) {
    let worker_batch_size = search_batch_candidates(thread_count);
    let mut state = SearchState::new(start.request);
    let started_at_ms = js_sys::Date::now();
    let mut searched_since_progress = 0;

    loop {
        if let Some(hit) = state.step(worker_batch_size) {
            let now_ms = js_sys::Date::now();
            let progress = state.progress();
            let pixels_per_second =
                pixels_per_second(progress.searched_pixels, started_at_ms, now_ms);
            post_event(
                scope,
                SearchWorkerEvent::Hit {
                    token_id: start.token_id,
                    progress,
                    pixels_per_second,
                    hit,
                },
            );
            return;
        }

        searched_since_progress += worker_batch_size;
        if searched_since_progress >= SEARCH_PROGRESS_CANDIDATES {
            let now_ms = js_sys::Date::now();
            let progress = state.progress();
            let pixels_per_second =
                pixels_per_second(progress.searched_pixels, started_at_ms, now_ms);
            post_event(
                scope,
                SearchWorkerEvent::Progress {
                    token_id: start.token_id,
                    progress,
                    pixels_per_second,
                },
            );
            searched_since_progress = 0;
        }
    }
}

fn install_message_handler(scope: DedicatedWorkerGlobalScope, thread_count: usize) {
    let handler_scope = scope.clone();
    let on_message =
        Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
            let message = Uint8Array::new(&event.data()).to_vec();

            match bincode::deserialize::<SearchWorkerStart>(&message) {
                Ok(start) => run_search(&handler_scope, start, thread_count),
                Err(error) => post_event(
                    &handler_scope,
                    SearchWorkerEvent::Error {
                        token_id: None,
                        message: format!("failed to parse search request: {error}"),
                    },
                ),
            }
        }));

    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    post_event(&scope, SearchWorkerEvent::Ready);
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    spawn_local(async move {
        let thread_count = hardware_concurrency();
        match JsFuture::from(wasm_bindgen_rayon::init_thread_pool(thread_count)).await {
            Ok(_) => install_message_handler(scope, thread_count),
            Err(error) => post_event(
                &scope,
                SearchWorkerEvent::Error {
                    token_id: None,
                    message: format!("failed to initialize rayon worker pool: {error:?}"),
                },
            ),
        }
    });
}

fn main() {}
