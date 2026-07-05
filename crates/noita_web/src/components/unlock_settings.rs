use crate::components::ui::SpellCard;
use leptos::prelude::*;
use leptos::task::spawn_local;
use noita_sim::Spell;
use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockFlagGroup {
    pub flag: &'static str,
    pub spells: Vec<Spell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PerkFlag {
    flag: &'static str,
    label: &'static str,
}

const NO_MORE_SHUFFLE_WANDS_FLAG: &str = "perk_picked_no_more_shuffle";
const HIDDEN_SPELL_UNLOCK_FLAGS: &[&str] = &["card_unlocked_infinite"];

fn is_visible_spell_unlock_flag(flag: &str) -> bool {
    !HIDDEN_SPELL_UNLOCK_FLAGS.contains(&flag)
}

pub fn unlock_flag_groups() -> Vec<UnlockFlagGroup> {
    let mut groups: Vec<UnlockFlagGroup> = Vec::new();
    for spell in Spell::ALL {
        let Some(flag) = spell.unlock_flag() else {
            continue;
        };
        if !is_visible_spell_unlock_flag(flag) {
            continue;
        }
        if let Some(group) = groups.iter_mut().find(|group| group.flag == flag) {
            group.spells.push(spell);
        } else {
            groups.push(UnlockFlagGroup {
                flag,
                spells: vec![spell],
            });
        }
    }
    groups
}

fn perk_flags() -> Vec<PerkFlag> {
    vec![PerkFlag {
        flag: NO_MORE_SHUFFLE_WANDS_FLAG,
        label: "No More Shuffle",
    }]
}

fn importable_flags() -> Vec<&'static str> {
    let mut flags = unlock_flag_groups()
        .into_iter()
        .map(|group| group.flag)
        .collect::<Vec<_>>();
    flags.extend(perk_flags().into_iter().map(|perk| perk.flag));
    flags
}

pub fn all_unlock_flags() -> Vec<String> {
    unlock_flag_groups()
        .into_iter()
        .map(|group| group.flag.to_string())
        .collect()
}

pub fn normalize_unlock_flags(flags: Vec<String>) -> Vec<String> {
    let selected: BTreeSet<String> = flags.into_iter().collect();
    importable_flags()
        .into_iter()
        .filter_map(|flag| selected.contains(flag).then(|| flag.to_string()))
        .collect()
}

#[wasm_bindgen(inline_js = r#"
export async function droppedFileNames(dataTransfer) {
  const names = [];

  async function readEntry(entry) {
    if (!entry) return;
    if (entry.isFile) {
      names.push(entry.name);
      return;
    }
    if (!entry.isDirectory) return;
    const reader = entry.createReader();
    while (true) {
      const batch = await new Promise((resolve, reject) => reader.readEntries(resolve, reject));
      if (!batch.length) break;
      for (const child of batch) {
        await readEntry(child);
      }
    }
  }

  if (dataTransfer.items && dataTransfer.items.length) {
    for (const item of dataTransfer.items) {
      const entry = item.webkitGetAsEntry ? item.webkitGetAsEntry() : null;
      if (entry) {
        await readEntry(entry);
      } else if (item.kind === "file") {
        const file = item.getAsFile();
        if (file) names.push(file.name);
      }
    }
  }

  if (!names.length && dataTransfer.files) {
    for (const file of dataTransfer.files) names.push(file.name);
  }

  return names;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = droppedFileNames)]
    async fn dropped_file_names(data_transfer: web_sys::DataTransfer) -> JsValue;
}

fn js_names(value: JsValue) -> Vec<String> {
    js_sys::Array::from(&value)
        .iter()
        .filter_map(|value| value.as_string())
        .collect()
}

fn file_names(files: web_sys::FileList) -> Vec<String> {
    let mut names = Vec::new();
    for idx in 0..files.length() {
        if let Some(file) = files.get(idx) {
            names.push(file.name());
        }
    }
    names
}

fn apply_named_flags(names: Vec<String>, unlock_flags: RwSignal<Vec<String>>) -> String {
    let known: BTreeSet<&'static str> = importable_flags().into_iter().collect();
    let selected = names
        .into_iter()
        .filter(|name| known.contains(name.as_str()))
        .collect::<Vec<_>>();
    let next = normalize_unlock_flags(selected);
    if next.is_empty() {
        return "No Noita spell unlock or supported generation flag files recognized. Drop save00/persistent/flags or select that folder."
            .to_string();
    }
    let count = next.len();
    unlock_flags.set(next);
    format!("Imported {count} recognized flag files.")
}

fn set_flag(unlock_flags: RwSignal<Vec<String>>, flag: &'static str, enabled: bool) {
    let mut next = unlock_flags.get_untracked();
    if enabled {
        if !next.iter().any(|item| item == flag) {
            next.push(flag.to_string());
        }
    } else {
        next.retain(|item| item != flag);
    }
    next = normalize_unlock_flags(next);
    unlock_flags.set(next);
}

#[component]
fn SpellUnlockRow(group: UnlockFlagGroup, unlock_flags: RwSignal<Vec<String>>) -> impl IntoView {
    let flag = group.flag;
    let spells = group.spells;
    view! {
        <label class="unlock-flag-row">
            <input
                type="checkbox"
                prop:checked=move || unlock_flags.with(|flags| flags.iter().any(|item| item == flag))
                on:change=move |ev| set_flag(unlock_flags, flag, event_target_checked(&ev))
            />
            <span class="unlock-flag-copy">
                <b>{flag}</b>
                <span class="unlock-spell-icons">
                    <For each=move || spells.clone() key=|spell| spell.id() let:spell>
                        <SpellCard spell boxed=false />
                    </For>
                </span>
            </span>
        </label>
    }
}

#[component]
fn PerkFlagRow(perk: PerkFlag, unlock_flags: RwSignal<Vec<String>>) -> impl IntoView {
    let flag = perk.flag;
    let label = perk.label;
    view! {
        <label class="unlock-flag-row unlock-perk-row">
            <input
                type="checkbox"
                prop:checked=move || unlock_flags.with(|flags| flags.iter().any(|item| item == flag))
                on:change=move |ev| set_flag(unlock_flags, flag, event_target_checked(&ev))
            />
            <span class="unlock-flag-copy">
                <b>{label}</b>
                <small>{flag}</small>
            </span>
        </label>
    }
}

#[component]
pub fn UnlockSettings(unlock_flags: RwSignal<Vec<String>>) -> impl IntoView {
    let folder_input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(input) = folder_input.get() {
            let _ = input.set_attribute("webkitdirectory", "");
            let _ = input.set_attribute("directory", "");
        }
    });

    let import_status = RwSignal::new(
        "Drop save00/persistent/flags here, or open the folder picker. Missing files become locked."
            .to_string(),
    );
    let drag_active = RwSignal::new(false);
    let spell_groups = unlock_flag_groups();
    let perk_groups = perk_flags();
    let total = spell_groups.len() + perk_groups.len();
    let selected_count = move || unlock_flags.with(Vec::len);
    let all_spell_flags = all_unlock_flags();
    let spell_flag_names = spell_groups
        .iter()
        .map(|group| group.flag)
        .collect::<Vec<_>>();
    let all_flags = {
        let spell_flag_names = spell_flag_names.clone();
        let all_spell_flags = all_spell_flags.clone();
        move |_| {
            let mut next = unlock_flags.get_untracked();
            next.retain(|flag| !spell_flag_names.iter().any(|spell_flag| flag == spell_flag));
            next.extend(all_spell_flags.iter().cloned());
            let count = all_spell_flags.len();
            unlock_flags.set(normalize_unlock_flags(next));
            import_status.set(format!("Enabled all {count} spell unlock flags."));
        }
    };
    let no_flags = move |_| {
        let mut next = unlock_flags.get_untracked();
        next.retain(|flag| !spell_flag_names.iter().any(|spell_flag| flag == spell_flag));
        unlock_flags.set(normalize_unlock_flags(next));
        import_status.set("Disabled every spell unlock flag.".to_string());
    };
    #[allow(unused_variables)]
    let import_names = Callback::new(move |names: Vec<String>| {
        import_status.set(apply_named_flags(names, unlock_flags));
        drag_active.set(false);
    });

    view! {
        <fieldset class="field-set unlock-field-set">
            <details class="unlock-menu">
                <summary>
                    <span class="unlock-summary-copy">
                        <span class="panel-title">"Save Flags"</span>
                        <small>"Unlocks & Perks"</small>
                    </span>
                    <span class="unlock-summary-meta">
                        <b>{move || format!("{}/{}", selected_count(), total)}</b>
                        <span class="unlock-caret" aria-hidden="true"></span>
                    </span>
                </summary>
                <div class="unlock-body">
                    <div
                        class:dragging=move || drag_active.get()
                        class="unlock-dropzone"
                        on:dragover=move |ev| {
                            ev.prevent_default();
                            drag_active.set(true);
                        }
                        on:dragleave=move |_| drag_active.set(false)
                        on:drop=move |ev| {
                            ev.prevent_default();
                            if let Some(data_transfer) = ev.data_transfer() {
                                let import_names = import_names;
                                spawn_local(async move {
                                    import_names.run(js_names(dropped_file_names(data_transfer).await));
                                });
                            } else {
                                import_status.set("Drop did not include files. Drop the flags folder itself.".to_string());
                                drag_active.set(false);
                            }
                        }
                    >
                        <p>"Drop Noita flag files"</p>
                        <span>"%USERPROFILE%/AppData/LocalLow/Nolla_Games_Noita/save00/persistent/flags"</span>
                        <input
                            id="unlock-folder-input"
                            class="sr-only"
                            type="file"
                            multiple
                            node_ref=folder_input
                            on:change=move |ev| {
                                let input = event_target::<web_sys::HtmlInputElement>(&ev);
                                if let Some(files) = input.files() {
                                    import_names.run(file_names(files));
                                } else {
                                    import_status.set("Folder picker returned no files.".to_string());
                                }
                            }
                        />
                        <label class="btn btn-ghost" for="unlock-folder-input">"choose flags folder"</label>
                    </div>
                    <p class="unlock-status">{move || import_status.get()}</p>
                    <section class="unlock-section" aria-labelledby="spell-unlocks-heading">
                        <h3 id="spell-unlocks-heading" class="unlock-section-heading">"Spell Unlocks"</h3>
                        <div class="unlock-actions">
                            <button type="button" class="btn btn-ghost" on:click=all_flags>"all unlocked"</button>
                            <button type="button" class="btn btn-ghost" on:click=no_flags>"no unlocks"</button>
                        </div>
                        <div class="unlock-flag-list">
                            <For each=move || spell_groups.clone() key=|group| group.flag let:group>
                                <SpellUnlockRow group unlock_flags />
                            </For>
                        </div>
                    </section>
                    <section class="unlock-section unlock-perk-section" aria-labelledby="perk-flags-heading">
                        <h3 id="perk-flags-heading" class="unlock-section-heading">"Perks"</h3>
                        <div class="unlock-flag-list unlock-perk-list">
                            <For each=move || perk_groups.clone() key=|perk| perk.flag let:perk>
                                <PerkFlagRow perk unlock_flags />
                            </For>
                        </div>
                    </section>
                </div>
            </details>
        </fieldset>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_unlock_flags_exclude_hidden_infinite_but_keep_funky() {
        let flags = all_unlock_flags();

        assert!(flags.iter().any(|flag| flag == "card_unlocked_funky"));
        assert!(!flags.iter().any(|flag| flag == "card_unlocked_infinite"));
    }

    #[test]
    fn normalize_accepts_perks_and_drops_hidden_spell_unlocks() {
        let flags = normalize_unlock_flags(vec![
            "card_unlocked_infinite".to_string(),
            "card_unlocked_funky".to_string(),
            NO_MORE_SHUFFLE_WANDS_FLAG.to_string(),
        ]);

        assert_eq!(
            flags,
            vec![
                "card_unlocked_funky".to_string(),
                NO_MORE_SHUFFLE_WANDS_FLAG.to_string()
            ]
        );
    }
}
