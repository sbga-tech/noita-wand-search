use super::select_option::SelectOption;
use super::shared::{scroll_active_into_view, CHEVRON_DOWN};
use leptos::prelude::*;

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
