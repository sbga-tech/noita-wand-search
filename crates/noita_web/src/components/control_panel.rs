use crate::components::controls::{Combobox, Dropdown, SelectOption};
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
    #[serde(default = "default_predicate_amount")]
    pub amount: String,
    pub value: PredicateValue,
    pub excluded: bool,
}

fn default_predicate_amount() -> String {
    "1".to_string()
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
            amount: default_predicate_amount(),
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
                        let amount = parse_num::<usize>(&predicate.amount, "Amount")?.max(1);
                        WandFilterKind::SpellDeckRequirement {
                            value: spell,
                            amount,
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

fn numeric_condition_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("equals", "equals"),
        SelectOption::new("not_equals", "not equals"),
        SelectOption::new("greater_than", "greater than"),
        SelectOption::new("greater_than_or_equals", "greater than or equals"),
        SelectOption::new("less_than", "less than"),
        SelectOption::new("less_than_or_equals", "less than or equals"),
    ]
}

fn equality_condition_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("equals", "equals"),
        SelectOption::new("not_equals", "not equals"),
    ]
}

fn membership_condition_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("contains", "contains"),
        SelectOption::new("excludes", "excludes"),
    ]
}

fn comparison_to_value(comparison: &Comparison, membership: bool) -> &'static str {
    if membership {
        match comparison {
            Comparison::NotEquals => "excludes",
            _ => "contains",
        }
    } else {
        match comparison {
            Comparison::Equals => "equals",
            Comparison::NotEquals => "not_equals",
            Comparison::GreaterThan => "greater_than",
            Comparison::GreaterThanOrEquals => "greater_than_or_equals",
            Comparison::LessThan => "less_than",
            Comparison::LessThanOrEquals => "less_than_or_equals",
        }
    }
}

fn attribute_options() -> Vec<SelectOption> {
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
    .map(|variant| SelectOption::new(variant.value(), variant.label()))
    .collect()
}

fn spell_options() -> Vec<SelectOption> {
    Spell::ALL
        .iter()
        .map(|spell| {
            let name = spell.display_name("en");
            SelectOption::new(name, name)
        })
        .collect()
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
    let is_deck = move || attribute.get() == PredicateAttribute::SpellDeck;
    let amount_snapshot = move || {
        predicates.with(|items| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.amount.clone())
                .unwrap_or_else(|| "1".to_string())
        })
    };

    let attribute_selected = Signal::derive(move || attribute.get().value().to_string());
    let condition_options = Signal::derive(move || {
        let attr = attribute.get();
        match attr {
            PredicateAttribute::SpellDeck => membership_condition_options(),
            _ => match attr.constraint() {
                ValidationConstraint::Number | ValidationConstraint::Integer => {
                    numeric_condition_options()
                }
                _ => equality_condition_options(),
            },
        }
    });
    let condition_selected =
        Signal::derive(move || comparison_to_value(&comparison_snapshot(), is_deck()).to_string());

    let value_kind = Memo::new(move |_| match value_snapshot() {
        PredicateValue::Number(_) => 0u8,
        PredicateValue::Boolean(_) => 1,
        PredicateValue::String(_) => 2,
        PredicateValue::List(_) => 3,
    });
    let number_snapshot = move || match value_snapshot() {
        PredicateValue::Number(value) => value,
        _ => String::new(),
    };
    let bool_snapshot = move || matches!(value_snapshot(), PredicateValue::Boolean(true));
    let string_snapshot = move || match value_snapshot() {
        PredicateValue::String(value) => value,
        _ => String::new(),
    };
    let list_len_snapshot = move || match value_snapshot() {
        PredicateValue::List(values) => values.len(),
        _ => 0,
    };

    let value_view = move || {
        match value_kind.get() {
        0 => view! {
            <input
                class="predicate-value atlas-input"
                type="text"
                inputmode=move || if attribute.get().constraint() == ValidationConstraint::Integer { "numeric" } else { "decimal" }
                prop:value=number_snapshot
                placeholder=move || if attribute.get().constraint() == ValidationConstraint::Integer { "integer" } else { "number" }
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::Number(value); });
                }
            />
        }.into_any(),
        1 => view! {
            <Dropdown
                options=Signal::derive(|| vec![SelectOption::new("true", "true"), SelectOption::new("false", "false")])
                selected=Signal::derive(move || if bool_snapshot() { "true".to_string() } else { "false".to_string() })
                on_select=Callback::new(move |value: String| {
                    let value = value == "true";
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::Boolean(value); });
                })
                class="predicate-value"
            />
        }.into_any(),
        2 => view! {
            <Combobox
                options=Signal::derive(spell_options)
                value=Signal::derive(string_snapshot)
                on_input=Callback::new(move |value: String| {
                    predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.value = PredicateValue::String(value); });
                })
                placeholder="spell name"
                class="predicate-value"
            />
        }.into_any(),
        _ => view! {
            <input class="predicate-value atlas-input" prop:value=move || format!("{} values", list_len_snapshot()) disabled=true />
        }.into_any(),
    }
    };

    view! {
        <div class="predicate-row grid items-end gap-2 border-2 border-bronze p-2 bg-[rgba(10,8,6,0.6)] grid-cols-[8.5rem_minmax(0,1fr)_6.5rem_2.5rem]">
            <label class="field"><span class="field-label text-[0.6rem]">"Attribute"</span>
                <Dropdown
                    options=Signal::derive(attribute_options)
                    selected=attribute_selected
                    on_select=Callback::new(move |value: String| {
                        let attribute = PredicateAttribute::from_value(&value);
                        predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { *item = PredicateInput::new(id, attribute); });
                    })
                    class="predicate-attribute"
                />
            </label>
            <div class="predicate-group flex min-w-0 items-end gap-2">
                <label class="field w-[7.5rem] shrink-0"><span class="field-label text-[0.6rem]">"Condition"</span>
                    <Dropdown
                        options=condition_options
                        selected=condition_selected
                        on_select=Callback::new(move |value: String| {
                            let constraint = attribute.get_untracked().constraint();
                            let comparison = comparison_from_value(&value, constraint);
                            predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.comparison = comparison; });
                        })
                        class="predicate-condition"
                    />
                </label>
                <Show when=is_deck>
                    <label class="field w-[3rem] shrink-0"><span class="field-label text-[0.6rem]">"Amount"</span>
                        <input class="predicate-amount atlas-input w-full px-1 text-center" type="number" min="1" step="1" prop:value=amount_snapshot on:input=move |ev| {
                            let amount = event_target_value(&ev);
                            predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.amount = amount; });
                        } />
                    </label>
                </Show>
                <label class="field min-w-0 flex-1"><span class="field-label text-[0.6rem]">"Value"</span>{value_view}</label>
            </div>
            <label class="field"><span class="field-label text-[0.6rem]">"Then"</span>
                <Dropdown
                    options=Signal::derive(|| vec![SelectOption::new("include", "include"), SelectOption::new("exclude", "exclude")])
                    selected=Signal::derive(move || if excluded_snapshot() { "exclude".to_string() } else { "include".to_string() })
                    on_select=Callback::new(move |value: String| {
                        let excluded = value == "exclude";
                        predicates.update(|items| if let Some(item) = items.iter_mut().find(|item| item.id == id) { item.excluded = excluded; });
                    })
                    class="predicate-mode"
                />
            </label>
            <button type="button" class="remove-predicate btn btn-danger w-[2.5rem] px-0 text-lg" title="remove predicate" on:click=move |_| {
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
                        <label class="field"><span class="field-label">"Search type"</span>
                            <Dropdown
                                options=Signal::derive(|| vec![
                                    SelectOption::new("0", "EoE Wand"),
                                    SelectOption::new("1", "Taikasauva Wand"),
                                    SelectOption::new("2", "Tiny Drop"),
                                ])
                                selected=Signal::derive(move || match mode.get() {
                                    SearchMode::EoeWand => "0",
                                    SearchMode::TaikasauvaWand => "1",
                                    SearchMode::TinyDropWand => "2",
                                }.to_string())
                                on_select=Callback::new(move |value: String| {
                                    let selected = match value.as_str() { "1" => SearchMode::TaikasauvaWand, "2" => SearchMode::TinyDropWand, _ => SearchMode::EoeWand };
                                    mode.set(selected);
                                    if selected == SearchMode::TinyDropWand { x.set("14941".to_string()); y.set("18654".to_string()); }
                                })
                                class="search-mode"
                            />
                        </label>
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
            amount: "1".into(),
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
            amount: "1".into(),
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
            amount: "1".into(),
            value: PredicateValue::String("Circle of stillness".into()),
            excluded: false,
        }];

        assert!(validate_state(&state).is_ok());
    }

    #[test]
    fn validate_state_passes_deck_amount_into_filter() {
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput {
            id: 1,
            attribute: PredicateAttribute::SpellDeck,
            comparison: Comparison::Equals,
            amount: "3".into(),
            value: PredicateValue::String("Add mana".into()),
            excluded: false,
        }];

        let request = validate_state(&state).expect("deck filter should validate");
        let kind = &request.wand_filters.filters[0].kind;
        assert!(matches!(
            kind,
            WandFilterKind::SpellDeckRequirement { amount: 3, .. }
        ));
    }

    #[test]
    fn validate_state_clamps_blank_deck_amount_error() {
        let mut state = default_form_state();
        state.predicates = vec![PredicateInput {
            id: 1,
            attribute: PredicateAttribute::SpellDeck,
            comparison: Comparison::Equals,
            amount: "".into(),
            value: PredicateValue::String("Add mana".into()),
            excluded: false,
        }];

        let error = validate_state(&state).expect_err("blank amount should not parse");
        assert!(error.contains("Amount"));
    }
}
