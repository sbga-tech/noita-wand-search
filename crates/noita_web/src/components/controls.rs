//! Themed form controls that replace native `<select>`/`<datalist>` elements.
//!
//! [`Dropdown`] is a shadcn-style `Select`: a trigger button opening a popover
//! list; the current value is shown with a background highlight (no check mark).
//! [`Combobox`] is the shadcn *base* combobox: a text input with a chevron that
//! opens a filtered suggestion list — typing edits the value and narrows the
//! list, picking an entry commits it. Both support keyboard navigation
//! (arrows / Enter / Escape), pointer hover highlighting, click-outside
//! dismissal, and active-item scroll-into-view, all styled by `tailwind.css`.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

const CHEVRON_DOWN: &str = r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// A single selectable entry: the stable `value` used by callers and the
/// human-facing `label` shown in the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Keeps the highlighted option visible when navigating a scrollable list.
fn scroll_active_into_view(list: &web_sys::HtmlDivElement) {
    let Ok(Some(item)) = list.query_selector(".atlas-menu-item.highlighted") else {
        return;
    };
    let Ok(item) = item.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };
    let top = item.offset_top();
    let bottom = top + item.offset_height();
    let view_top = list.scroll_top();
    let view_bottom = view_top + list.client_height();
    if top < view_top {
        list.set_scroll_top(top);
    } else if bottom > view_bottom {
        list.set_scroll_top(bottom - list.client_height());
    }
}

/// Themed replacement for a native single-select `<select>` (shadcn `Select`).
///
/// `options` and `selected` are reactive; choosing an entry fires `on_select`
/// with its `value`. Opens on click or keyboard; closes on selection, Escape,
/// or an outside click. The current value carries a background highlight.
#[component]
pub fn Dropdown(
    /// Reactive list of choices.
    #[prop(into)]
    options: Signal<Vec<SelectOption>>,
    /// Reactive currently-selected value.
    #[prop(into)]
    selected: Signal<String>,
    /// Fired with the chosen option's value.
    on_select: Callback<String>,
    /// Extra classes appended to the trigger button.
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let active = RwSignal::new(0usize);
    let list_ref = NodeRef::<leptos::html::Div>::new();
    let trigger_class = format!("atlas-select {class}");

    let selected_label = move || {
        let current = selected.get();
        options.with(|opts| {
            opts.iter()
                .find(|opt| opt.value == current)
                .map(|opt| opt.label.clone())
                .unwrap_or(current)
        })
    };

    let open_menu = move || {
        let idx = options.with_untracked(|opts| {
            let current = selected.get_untracked();
            opts.iter()
                .position(|opt| opt.value == current)
                .unwrap_or(0)
        });
        active.set(idx);
        open.set(true);
    };

    let commit = move |value: String| {
        on_select.run(value);
        open.set(false);
    };

    Effect::new(move |_| {
        active.track();
        if open.get() {
            if let Some(list) = list_ref.get() {
                scroll_active_into_view(&list);
            }
        }
    });

    view! {
        <div class="atlas-combo" class:open=move || open.get()>
            <button
                type="button"
                class=trigger_class
                class:open=move || open.get()
                on:click=move |_| if open.get() { open.set(false) } else { open_menu() }
                on:keydown=move |ev| {
                    let key = ev.key();
                    if !open.get() {
                        if key == "ArrowDown" || key == "ArrowUp" || key == "Enter" || key == " " {
                            ev.prevent_default();
                            open_menu();
                        }
                        return;
                    }
                    match key.as_str() {
                        "Escape" => { ev.prevent_default(); open.set(false); }
                        "ArrowDown" => {
                            ev.prevent_default();
                            let len = options.with(Vec::len);
                            if len > 0 { active.update(|a| *a = (*a + 1).min(len - 1)); }
                        }
                        "ArrowUp" => { ev.prevent_default(); active.update(|a| *a = a.saturating_sub(1)); }
                        "Enter" | " " => {
                            ev.prevent_default();
                            let picked = options.with(|opts| opts.get(active.get()).map(|o| o.value.clone()));
                            if let Some(value) = picked { commit(value); }
                        }
                        _ => {}
                    }
                }
            >
                <span class="atlas-select-value">{selected_label}</span>
                <span class="atlas-select-icon" aria-hidden="true" inner_html=CHEVRON_DOWN></span>
            </button>
            <Show when=move || open.get()>
                <div class="atlas-menu-backdrop" on:mousedown=move |_| open.set(false)></div>
                <div class="atlas-popover">
                    <div class="atlas-menu" node_ref=list_ref role="listbox">
                        <For
                            each=move || options.get().into_iter().enumerate()
                            key=|(_, opt)| opt.value.clone()
                            let:entry
                        >
                            {
                                let (index, opt) = entry;
                                let value = opt.value.clone();
                                let is_selected = {
                                    let value = value.clone();
                                    move || selected.get() == value
                                };
                                view! {
                                    <button
                                        type="button"
                                        class="atlas-menu-item"
                                        class:highlighted=move || active.get() == index
                                        class:selected=is_selected
                                        role="option"
                                        on:mouseenter=move |_| active.set(index)
                                        on:mousedown=move |ev| { ev.prevent_default(); commit(value.clone()); }
                                    >
                                        <span class="atlas-menu-label">{opt.label.clone()}</span>
                                    </button>
                                }
                            }
                        </For>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Themed replacement for `<input list=…>` + `<datalist>` — the shadcn *base*
/// combobox: a text input with a chevron toggle.
///
/// The `value` is authoritative and directly editable; typing narrows `options`
/// (case-insensitive substring) and picking a suggestion commits it via
/// `on_input`. Opens on focus/typing/chevron; closes on selection, Escape, or an
/// outside click.
#[component]
pub fn Combobox(
    /// Reactive suggestion pool.
    #[prop(into)]
    options: Signal<Vec<SelectOption>>,
    /// Reactive current value (also the live query).
    #[prop(into)]
    value: Signal<String>,
    /// Fired with the new value on every edit or selection.
    on_input: Callback<String>,
    /// Placeholder shown when empty.
    #[prop(optional, into)]
    placeholder: String,
    /// Extra classes appended to the text input.
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let active = RwSignal::new(0usize);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let list_ref = NodeRef::<leptos::html::Div>::new();
    let input_class = format!("atlas-input atlas-combo-input {class}");

    // Case-insensitive substring filter over the live value, capped for size.
    let matches = move || {
        let needle = value.get().to_lowercase();
        options.with(|opts| {
            // When the text exactly matches an option you're browsing, not
            // searching, so show the whole list (the match stays highlighted).
            let exact = opts.iter().any(|opt| opt.label.to_lowercase() == needle);
            opts.iter()
                .filter(|opt| {
                    needle.is_empty() || exact || opt.label.to_lowercase().contains(&needle)
                })
                .take(100)
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let has_matches = move || !matches().is_empty();

    let commit = move |value: String| {
        on_input.run(value);
        open.set(false);
    };

    // Keep the highlighted match in view while navigating.
    Effect::new(move |_| {
        active.track();
        if open.get() {
            if let Some(list) = list_ref.get() {
                scroll_active_into_view(&list);
            }
        }
    });

    view! {
        <div class="atlas-combo" class:open=move || open.get()>
            <div class="atlas-combo-field">
                <input
                    class=input_class
                    node_ref=input_ref
                    type="text"
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:focus=move |_| { active.set(0); open.set(true); }
                    on:input=move |ev| { on_input.run(event_target_value(&ev)); active.set(0); open.set(true); }
                    on:keydown=move |ev| {
                        match ev.key().as_str() {
                            "Escape" => { ev.prevent_default(); open.set(false); }
                            "ArrowDown" => {
                                ev.prevent_default();
                                if !open.get() { open.set(true); return; }
                                let len = matches().len();
                                if len > 0 { active.update(|a| *a = (*a + 1).min(len - 1)); }
                            }
                            "ArrowUp" => { ev.prevent_default(); active.update(|a| *a = a.saturating_sub(1)); }
                            "Enter" => {
                                if open.get() {
                                    let hits = matches();
                                    if let Some(opt) = hits.get(active.get()).or_else(|| hits.first()) {
                                        ev.prevent_default();
                                        commit(opt.value.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                />
                <button
                    type="button"
                    class="atlas-combo-toggle"
                    tabindex="-1"
                    aria-label="toggle options"
                    on:mousedown=move |ev| {
                        ev.prevent_default();
                        if open.get() {
                            open.set(false);
                        } else {
                            active.set(0);
                            open.set(true);
                            if let Some(input) = input_ref.get() {
                                let _ = input.focus();
                            }
                        }
                    }
                >
                    <span class="atlas-select-icon" aria-hidden="true" inner_html=CHEVRON_DOWN></span>
                </button>
            </div>
            <Show when=move || open.get() && has_matches()>
                // `mousedown` (not `click`) so the input keeps focus long enough
                // for the option's own `mousedown` to register first.
                <div class="atlas-menu-backdrop" on:mousedown=move |_| open.set(false)></div>
                <div class="atlas-popover atlas-popover--combo">
                    <div class="atlas-menu" node_ref=list_ref role="listbox">
                        <For
                            each=move || matches().into_iter().enumerate()
                            key=|(_, opt)| opt.value.clone()
                            let:entry
                        >
                            {
                                let (index, opt) = entry;
                                let value_out = opt.value.clone();
                                let is_selected = {
                                    let value_out = value_out.clone();
                                    move || value.get() == value_out
                                };
                                view! {
                                    <button
                                        type="button"
                                        class="atlas-menu-item"
                                        class:highlighted=move || active.get() == index
                                        class:selected=is_selected
                                        role="option"
                                        on:mouseenter=move |_| active.set(index)
                                        on:mousedown=move |ev| { ev.prevent_default(); commit(value_out.clone()); }
                                    >
                                        <span class="atlas-menu-label">{opt.label.clone()}</span>
                                    </button>
                                }
                            }
                        </For>
                    </div>
                </div>
            </Show>
        </div>
    }
}
