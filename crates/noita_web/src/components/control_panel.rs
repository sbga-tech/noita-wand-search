use crate::components::unlock_settings::{all_unlock_flags, UnlockSettings};
use leptos::prelude::*;
use noita_sim::filters::{Comparison, FilterMode, WandFilter, WandFilterKind, WandFilterSet};
use noita_sim::search::{SearchMode, SearchRequest};
use noita_sim::validator::validate_search_request;
use noita_sim::{Spell, WandStat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PredicateAttribute {
    Capacity,
    Multicast,
    CastDelay,
    Reload,
    MaxMana,
    ManaRegen,
    Spread,
    Speed,
    Shuffle,
    SpellDeck,
    AlwaysCast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ValidationConstraint {
    Integer,
    Number,
    Boolean,
    SpellName,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PredicateValue {
    Number(String),
    Boolean(bool),
    #[serde(alias = "Spell")]
    String(String),
    List(Vec<PredicateValue>),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PredicateInput {
    pub id: u64,
    pub attribute: PredicateAttribute,
    pub comparison: Comparison,
    pub value: PredicateValue,
    pub excluded: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FormState {
    pub seed: String,
    pub ng: String,
    pub x: String,
    pub y: String,
    pub mode: SearchMode,
    pub predicates: Vec<PredicateInput>,
    pub unlock_flags: Vec<String>,
}

const FORM_STORAGE_KEY: &str = "noita-wand-atlas-form-state";

impl PredicateAttribute {
    fn constraint(self) -> ValidationConstraint {
        match self {
            Self::Capacity | Self::Multicast | Self::MaxMana | Self::ManaRegen | Self::Spread => {
                ValidationConstraint::Integer
            }
            Self::Shuffle => ValidationConstraint::Boolean,
            Self::SpellDeck | Self::AlwaysCast => ValidationConstraint::SpellName,
            Self::CastDelay | Self::Reload | Self::Speed => ValidationConstraint::Number,
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "multicast" => Self::Multicast,
            "cast_delay" => Self::CastDelay,
            "reload" => Self::Reload,
            "max_mana" => Self::MaxMana,
            "mana_regen" => Self::ManaRegen,
            "spread" => Self::Spread,
            "speed" => Self::Speed,
            "shuffle" => Self::Shuffle,
            "spell_deck" => Self::SpellDeck,
            "always_cast" => Self::AlwaysCast,
            _ => Self::Capacity,
        }
    }

    fn value(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Multicast => "multicast",
            Self::CastDelay => "cast_delay",
            Self::Reload => "reload",
            Self::MaxMana => "max_mana",
            Self::ManaRegen => "mana_regen",
            Self::Spread => "spread",
            Self::Speed => "speed",
            Self::Shuffle => "shuffle",
            Self::SpellDeck => "spell_deck",
            Self::AlwaysCast => "always_cast",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Multicast => "multicast",
            Self::CastDelay => "cast delay",
            Self::Reload => "reload",
            Self::MaxMana => "max mana",
            Self::ManaRegen => "mana regen",
            Self::Spread => "spread",
            Self::Speed => "speed",
            Self::Shuffle => "shuffle",
            Self::SpellDeck => "spell deck",
            Self::AlwaysCast => "always cast",
        }
    }

    fn as_wand_stat(self) -> Option<WandStat> {
        match self {
            Self::Capacity => Some(WandStat::Capacity),
            Self::Multicast => Some(WandStat::Multicast),
            Self::CastDelay => Some(WandStat::CastDelay),
            Self::Reload => Some(WandStat::Reload),
            Self::MaxMana => Some(WandStat::MaxMana),
            Self::ManaRegen => Some(WandStat::ManaRegen),
            Self::Spread => Some(WandStat::Spread),
            Self::Speed => Some(WandStat::Speed),
            _ => None,
        }
    }

    fn default_predicate(self, id: u64) -> PredicateInput {
        let (comparison, value) = match self {
            Self::Capacity => (
                Comparison::GreaterThanOrEquals,
                PredicateValue::Number("26".into()),
            ),
            Self::Multicast => (Comparison::Equals, PredicateValue::Number("1".into())),
            Self::CastDelay => (
                Comparison::LessThanOrEquals,
                PredicateValue::Number("0".into()),
            ),
            Self::Reload => (Comparison::LessThan, PredicateValue::Number("0.05".into())),
            Self::MaxMana => (
                Comparison::GreaterThan,
                PredicateValue::Number("500".into()),
            ),
            Self::ManaRegen => (
                Comparison::GreaterThan,
                PredicateValue::Number("1000".into()),
            ),
            Self::Spread => (Comparison::Equals, PredicateValue::Number("0".into())),
            Self::Speed => (
                Comparison::GreaterThanOrEquals,
                PredicateValue::Number("1".into()),
            ),
            Self::Shuffle => (Comparison::Equals, PredicateValue::Boolean(false)),
            Self::SpellDeck => (
                Comparison::Equals,
                PredicateValue::String("Add mana".into()),
            ),
            Self::AlwaysCast => (
                Comparison::Equals,
                PredicateValue::String("Add mana".into()),
            ),
        };
        PredicateInput {
            id,
            attribute: self,
            comparison,
            value,
            excluded: false,
        }
    }
}

impl PredicateInput {
    pub fn new(id: u64, attribute: PredicateAttribute) -> Self {
        attribute.default_predicate(id)
    }
}

pub fn default_form_state() -> FormState {
    FormState {
        seed: "1".into(),
        ng: "0".into(),
        x: "0".into(),
        y: "0".into(),
        mode: SearchMode::EoeWand,
        predicates: vec![PredicateInput::new(1, PredicateAttribute::Capacity)],
        unlock_flags: all_unlock_flags(),
    }
}

fn normalize_unlock_flags(flags: Vec<String>) -> Vec<String> {
    let selected = flags.into_iter().collect::<std::collections::BTreeSet<_>>();
    all_unlock_flags()
        .into_iter()
        .filter(|flag| selected.contains(flag))
        .collect()
}

fn normalize_form_state(mut state: FormState) -> FormState {
    state.unlock_flags = normalize_unlock_flags(state.unlock_flags);
    state
}

fn load_form_state() -> Option<FormState> {
    let storage = window().local_storage().ok().flatten()?;
    if let Some(raw) = storage.get_item(FORM_STORAGE_KEY).ok().flatten() {
        if let Ok(state) = serde_json::from_str::<FormState>(&raw) {
            return Some(normalize_form_state(state));
        }
    }
    None
}

fn save_form_state(state: &FormState) {
    if let Ok(Some(storage)) = window().local_storage() {
        if let Ok(raw) = serde_json::to_string(state) {
            let _ = storage.set_item(FORM_STORAGE_KEY, &raw);
        }
    }
}

pub fn apply_mode_side_effects(state: &mut FormState, mode: SearchMode) {
    state.mode = mode;
    if mode == SearchMode::TinyDropWand {
        state.x = "14941".into();
        state.y = "18654".into();
    }
}

fn parse_num<T: std::str::FromStr>(value: &str, label: &str) -> Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("Invalid number in {label}."))
}

fn parse_finite_f64(value: &str, label: &str) -> Result<f64, String> {
    let number = parse_num::<f64>(value, label)?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(format!("Invalid number in {label}."))
    }
}

fn parse_integer_value(value: &str, label: &str) -> Result<f64, String> {
    Ok(parse_finite_f64(value, label)?.floor())
}

pub fn validate_state(state: &FormState) -> Result<SearchRequest, String> {
    let seed = parse_num::<u32>(&state.seed, "Seed")?;
    let ng = parse_num::<u32>(&state.ng, "NG+")?;
    let start_x = parse_finite_f64(&state.x, "x")?;
    let start_y = parse_finite_f64(&state.y, "y")?;
    let mut wand_filters = WandFilterSet::default();
    for predicate in &state.predicates {
        match (&predicate.attribute.constraint(), &predicate.value) {
            (ValidationConstraint::Integer, PredicateValue::Number(value))
            | (ValidationConstraint::Number, PredicateValue::Number(value)) => {
                let stat = predicate
                    .attribute
                    .as_wand_stat()
                    .unwrap_or(WandStat::Capacity);
                let value = if predicate.attribute.constraint() == ValidationConstraint::Integer {
                    parse_integer_value(value, "Value")?
                } else {
                    parse_finite_f64(value, "Value")?
                };
                wand_filters.filters.push(WandFilter {
                    mode: if predicate.excluded {
                        FilterMode::Exclude
                    } else {
                        FilterMode::Include
                    },
                    kind: WandFilterKind::Stat {
                        stat,
                        comparison: predicate.comparison.clone(),
                        value,
                    },
                });
            }
            (ValidationConstraint::Boolean, PredicateValue::Boolean(value)) => {
                wand_filters.filters.push(WandFilter {
                    mode: if predicate.excluded {
                        FilterMode::Exclude
                    } else {
                        FilterMode::Include
                    },
                    kind: WandFilterKind::Shuffle {
                        comparison: predicate.comparison.clone(),
                        value: *value,
                    },
                });
            }
            (ValidationConstraint::SpellName, PredicateValue::String(value)) => {
                let spell = Spell::from_display_name(value, "en")
                    .ok_or_else(|| "Invalid spell!".to_string())?;
                let mut mode = if predicate.excluded {
                    FilterMode::Exclude
                } else {
                    FilterMode::Include
                };
                let kind = match predicate.attribute {
                    PredicateAttribute::AlwaysCast => WandFilterKind::AlwaysCast {
                        comparison: predicate.comparison.clone(),
                        value: spell,
                    },
                    _ => {
                        if predicate.comparison == Comparison::NotEquals {
                            mode = match mode {
                                FilterMode::Include => FilterMode::Exclude,
                                FilterMode::Exclude => FilterMode::Include,
                            };
                        }
                        WandFilterKind::SpellDeckRequirement {
                            value: spell,
                            amount: 1,
                        }
                    }
                };
                wand_filters.filters.push(WandFilter { mode, kind });
            }
            _ => {}
        }
    }
    let request = SearchRequest {
        seed,
        ng,
        start_x,
        start_y,
        mode: state.mode,
        wand_filters,
        unlock_flags: Some(state.unlock_flags.clone()),
    };
    validate_search_request(&request).map_err(|error| error.to_string())?;
    Ok(request)
}

pub fn submit_label(_mode: SearchMode) -> &'static str {
    "Seek Big Wands"
}

fn numeric_condition_options(selected: &Comparison) -> impl IntoView + use<> {
    view! {
        <option value="equals" selected=matches!(selected, Comparison::Equals)>"equals"</option>
        <option value="not_equals" selected=matches!(selected, Comparison::NotEquals)>"not equals"</option>
        <option value="greater_than" selected=matches!(selected, Comparison::GreaterThan)>"greater than"</option>
        <option value="greater_than_or_equals" selected=matches!(selected, Comparison::GreaterThanOrEquals)>"greater than or equals"</option>
        <option value="less_than" selected=matches!(selected, Comparison::LessThan)>"less than"</option>
        <option value="less_than_or_equals" selected=matches!(selected, Comparison::LessThanOrEquals)>"less than or equals"</option>
    }
}

fn equality_condition_options(selected: &Comparison) -> impl IntoView + use<> {
    view! {
        <option value="equals" selected=matches!(selected, Comparison::Equals)>"equals"</option>
        <option value="not_equals" selected=matches!(selected, Comparison::NotEquals)>"not equals"</option>
    }
}

fn membership_condition_options(selected: &Comparison) -> impl IntoView + use<> {
    view! {
        <option value="contains" selected=matches!(selected, Comparison::Equals)>"contains"</option>
        <option value="excludes" selected=matches!(selected, Comparison::NotEquals)>"excludes"</option>
    }
}

fn comparison_from_value(value: &str, constraint: ValidationConstraint) -> Comparison {
    match (value, constraint) {
        ("not_equals", _) | ("excludes", _) => Comparison::NotEquals,
        ("greater_than", ValidationConstraint::Number | ValidationConstraint::Integer) => {
            Comparison::GreaterThan
        }
        (
            "greater_than_or_equals",
            ValidationConstraint::Number | ValidationConstraint::Integer,
        ) => Comparison::GreaterThanOrEquals,
        ("less_than", ValidationConstraint::Number | ValidationConstraint::Integer) => {
            Comparison::LessThan
        }
        ("less_than_or_equals", ValidationConstraint::Number | ValidationConstraint::Integer) => {
            Comparison::LessThanOrEquals
        }
        _ => Comparison::Equals,
    }
}

fn predicate_row(id: u64, predicates: RwSignal<Vec<PredicateInput>>) -> impl IntoView {
    let attribute = Memo::new(move |_| {
        predicates.with(|items| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.attribute)
                .unwrap_or(PredicateAttribute::Capacity)
        })
    });
    let comparison_snapshot = move || {
        predicates.with(|items| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.comparison.clone())
                .unwrap_or(Comparison::Equals)
        })
    };
    let value_snapshot = move || {
        predicates.with(|items| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.value.clone())
                .unwrap_or(PredicateValue::Number("0".into()))
        })
    };
    let excluded_snapshot = move || {
        predicates.with(|items| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.excluded)
                .unwrap_or(false)
        })
    };

    let attr_options = move || {
        let attr = attribute.get();
        [
            PredicateAttribute::Capacity,
            PredicateAttribute::Multicast,
            PredicateAttribute::CastDelay,
            PredicateAttribute::Reload,
            PredicateAttribute::MaxMana,
            PredicateAttribute::ManaRegen,
            PredicateAttribute::Spread,
            PredicateAttribute::Speed,
            PredicateAttribute::Shuffle,
            PredicateAttribute::SpellDeck,
            PredicateAttribute::AlwaysCast,
        ]
        .into_iter()
        .map(|variant| {
            view! { <option value=variant.value() selected=variant == attr>{variant.label()}</option> }
        })
        .collect_view()
    };

    let condition_view = move || {
        let attr = attribute.get();
        let current = comparison_snapshot();
        match attr {
            PredicateAttribute::SpellDeck => membership_condition_options(&current).into_any(),
            _ => match attr.constraint() {
                ValidationConstraint::Number | ValidationConstraint::Integer => {
                    numeric_condition_options(&current).into_any()
                }
                _ => equality_condition_options(&current).into_any(),
            },
        }
    };

    let value_view = move || {
        let attr = attribute.get();
        match value_snapshot() {
            PredicateValue::Number(value) => {
                let inputmode = if attr.constraint() == ValidationConstraint::Integer {
                    "numeric"
                } else {
                    "decimal"
                };
                view! {
                    <input class="predicate-value atlas-input" type="text" inputmode=inputmode prop:value=value placeholder=if inputmode == "numeric" { "integer" } else { "number" } on:input=move |ev| {
                        let value = event_target_value(&ev);
                        predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::Number(value); });
                    } />
                }.into_any()
            },
            PredicateValue::Boolean(value) => view! {
                <select class="predicate-value atlas-input" on:change=move |ev| {
                    let value = event_target_value(&ev) == "true";
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::Boolean(value); });
                }>
                    <option value="true" selected=value>"true"</option>
                    <option value="false" selected=!value>"false"</option>
                </select>
            }.into_any(),
            PredicateValue::String(value) => view! {
                <input class="predicate-value atlas-input" prop:value=value list="spell-names" placeholder="spell name" on:input=move |ev| {
                    let value = event_target_value(&ev);
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::String(value); });
                } />
            }.into_any(),
            PredicateValue::List(values) => view! {
                <input class="predicate-value atlas-input" value=format!("{} values", values.len()) disabled=true />
            }.into_any(),
        }
    };

    view! {
        <div class="predicate-row">
            <label class="field"><span class="field-label">"Attribute"</span>
                <select class="predicate-attribute atlas-input" on:change=move |ev| {
                    let attribute = PredicateAttribute::from_value(&event_target_value(&ev));
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { *item = PredicateInput::new(id, attribute); });
                }>
                    {attr_options}
                </select>
            </label>
            <label class="field"><span class="field-label">"Condition"</span>
                <select class="predicate-condition atlas-input" on:change=move |ev| {
                    let constraint = attribute.get_untracked().constraint();
                    let comparison = comparison_from_value(&event_target_value(&ev), constraint);
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.comparison = comparison; });
                }>
                    {condition_view}
                </select>
            </label>
            <label class="field"><span class="field-label">"Value"</span>{value_view}</label>
            <label class="field"><span class="field-label">"Then"</span>
                <select class="predicate-mode atlas-input" on:change=move |ev| {
                    let excluded = event_target_value(&ev) == "exclude";
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.excluded = excluded; });
                }>
                    <option value="include" selected=!excluded_snapshot()>"include"</option>
                    <option value="exclude" selected=excluded_snapshot()>"exclude"</option>
                </select>
            </label>
            <button type="button" class="remove-predicate btn btn-danger" title="remove predicate" on:click=move |_| {
                predicates.update(|items| items.retain(|item| item.id != id));
            }>"×"</button>
        </div>
    }
}

fn next_predicate_id(state: &FormState) -> u64 {
    state
        .predicates
        .iter()
        .map(|predicate| predicate.id)
        .max()
        .unwrap_or(0)
        + 1
}

#[component]
pub fn ControlPanel(
    on_search: Callback<FormState>,
    on_cancel: Callback<()>,
    searching: ReadSignal<bool>,
) -> impl IntoView {
    let initial_state = default_form_state();
    let initial_next_predicate_id = next_predicate_id(&initial_state);

    let seed = RwSignal::new(initial_state.seed);
    let ng = RwSignal::new(initial_state.ng);
    let x = RwSignal::new(initial_state.x);
    let y = RwSignal::new(initial_state.y);
    let mode = RwSignal::new(initial_state.mode);
    let next_id = RwSignal::new(initial_next_predicate_id);
    let predicates = RwSignal::new(initial_state.predicates);
    let unlock_flags = RwSignal::new(initial_state.unlock_flags);
    let storage_ready = RwSignal::new(false);

    Effect::new(move |_| {
        if storage_ready.get_untracked() {
            return;
        }
        if let Some(state) = load_form_state() {
            let restored_next_id = next_predicate_id(&state);
            seed.set(state.seed);
            ng.set(state.ng);
            x.set(state.x);
            y.set(state.y);
            mode.set(state.mode);
            predicates.set(state.predicates);
            unlock_flags.set(state.unlock_flags);
            next_id.set(restored_next_id);
        }
        storage_ready.set(true);
    });

    Effect::new(move |_| {
        if !storage_ready.get() {
            return;
        }
        save_form_state(&FormState {
            seed: seed.get(),
            ng: ng.get(),
            x: x.get(),
            y: y.get(),
            mode: mode.get(),
            predicates: predicates.get(),
            unlock_flags: unlock_flags.get(),
        });
    });

    let form_state = move || FormState {
        seed: seed.get(),
        ng: ng.get(),
        x: x.get(),
        y: y.get(),
        mode: mode.get(),
        predicates: predicates.get(),
        unlock_flags: unlock_flags.get(),
    };

    let zero = move |_| {
        x.set("0".to_string());
        y.set("0".to_string());
    };
    let add_predicate = move |_| {
        let id = next_id.get_untracked();
        next_id.set(id + 1);
        predicates
            .update(|items| items.push(PredicateInput::new(id, PredicateAttribute::Capacity)));
    };
    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_search.run(form_state());
    };
    let cancel = move |_| {
        on_cancel.run(());
    };

    let ids = move || {
        predicates
            .get()
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>()
    };

    view! {
        <section class="panel controls">
            <form class="atlas-form" on:submit=submit>
                <fieldset class="field-set">
                    <legend>"Origin"</legend>
                    <div class="origin-grid">
                        <label class="field"><span class="field-label">"Seed"</span><input id="seed" name="seed" class="atlas-input" type="text" inputmode="numeric" prop:value=move || seed.get() on:input=move |ev| seed.set(event_target_value(&ev)) /></label>
                        <label class="field"><span class="field-label">"NG+"</span><input id="ng" name="ng" class="atlas-input" type="text" inputmode="numeric" prop:value=move || ng.get() on:input=move |ev| ng.set(event_target_value(&ev)) /></label>
                        <label class="field"><span class="field-label">"Search type"</span><select id="search_mode" name="search_mode" class="atlas-input" on:change=move |ev| {
                            let selected = match event_target_value(&ev).as_str() { "1" => SearchMode::TaikasauvaWand, "2" => SearchMode::TinyDropWand, _ => SearchMode::EoeWand };
                            mode.set(selected);
                            if selected == SearchMode::TinyDropWand { x.set("14941".to_string()); y.set("18654".to_string()); }
                        }><option value="0" selected=move || mode.get() == SearchMode::EoeWand>"EoE Wand"</option><option value="1" selected=move || mode.get() == SearchMode::TaikasauvaWand>"Taikasauva Wand"</option><option value="2" selected=move || mode.get() == SearchMode::TinyDropWand>"Tiny Drop"</option></select></label>
                    </div>
                    <div class="coords-grid">
                        <label class="field"><span class="field-label">"x"</span><input id="x" name="x" class="atlas-input" type="text" inputmode="numeric" prop:value=move || x.get() on:input=move |ev| x.set(event_target_value(&ev)) /></label>
                        <label class="field"><span class="field-label">"y"</span><input id="y" name="y" class="atlas-input" type="text" inputmode="numeric" prop:value=move || y.get() on:input=move |ev| y.set(event_target_value(&ev)) /></label>
                        <button type="button" class="reset-origin btn btn-ghost" on:click=zero>"reset to origin"</button>
                    </div>
                </fieldset>
                <fieldset class="field-set">
                    <legend>"Predicates"</legend>
                    <div class="predicate-list">
                        <Show when=move || predicates.with(Vec::is_empty)>
                            <p class="predicate-empty">"No predicates — every generated wand matches."</p>
                        </Show>
                        <For each=ids key=|id| *id let:id>
                            {predicate_row(id, predicates)}
                        </For>
                    </div>
                    <button type="button" class="add-predicate btn btn-ghost" on:click=add_predicate>"+ add predicate"</button>
                </fieldset>
                <UnlockSettings unlock_flags />
                <div class="action-row">
                    <button id="true_knowledge_button" type="submit" class="btn btn-primary">{move || submit_label(mode.get())}</button>
                    <button id="cancel_button" type="button" class="cancel-button btn btn-ghost" prop:disabled=move || !searching.get() on:click=cancel>"cancel"</button>
                </div>
            </form>
            <datalist id="spell-names">
                {Spell::ALL.iter().map(|spell| view! { <option value=spell.display_name("en")></option> }).collect_view()}
            </datalist>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_state_rejects_out_of_range_capacity() {
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput {
            id: 1,
            attribute: PredicateAttribute::Capacity,
            comparison: Comparison::GreaterThan,
            value: PredicateValue::Number("61".into()),
            excluded: false,
        }];

        let error = validate_state(&state).expect_err("capacity > 61 should not search");
        assert!(error.contains("capacity filter is outside the possible range"));
    }

    #[test]
    fn validate_state_rejects_static_projectile_deck_filter() {
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput {
            id: 1,
            attribute: PredicateAttribute::SpellDeck,
            comparison: Comparison::Equals,
            value: PredicateValue::String("Circle of stillness".into()),
            excluded: false,
        }];

        let error =
            validate_state(&state).expect_err("static projectile deck filter should not search");
        assert!(error.contains("spell deck cannot contain Circle of stillness"));
    }

    #[test]
    fn validate_state_accepts_static_projectile_always_cast_filter() {
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput {
            id: 1,
            attribute: PredicateAttribute::AlwaysCast,
            comparison: Comparison::Equals,
            value: PredicateValue::String("Circle of stillness".into()),
            excluded: false,
        }];

        assert!(validate_state(&state).is_ok());
    }
}
