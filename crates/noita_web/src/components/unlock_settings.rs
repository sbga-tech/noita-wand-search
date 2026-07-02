use leptos::prelude::*;
use leptos::task::spawn_local;
use noita_sim::Spell;
use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockFlagGroup {
    pub flag: &'static str,
    pub spells: Vec<&'static str>,
}

pub fn unlock_flag_groups() -> Vec<UnlockFlagGroup> {
    let mut groups: Vec<UnlockFlagGroup> = Vec::new();
    for spell in Spell::ALL {
        let Some(flag) = spell.unlock_flag() else {
            continue;
        };
        let spell_name = spell.display_name("en");
        if let Some(group) = groups.iter_mut().find(|group| group.flag == flag) {
            group.spells.push(spell_name);
        } else {
            groups.push(UnlockFlagGroup {
                flag,
                spells: vec![spell_name],
            });
        }
    }
    groups
}

pub fn all_unlock_flags() -> Vec<String> {
    unlock_flag_groups()
        .into_iter()
        .map(|group| group.flag.to_string())
        .collect()
}

fn normalize_unlock_flags(flags: Vec<String>) -> Vec<String> {
    let selected: BTreeSet<String> = flags.into_iter().collect();
    unlock_flag_groups()
        .into_iter()
        .filter_map(|group| {
            selected
                .contains(group.flag)
                .then(|| group.flag.to_string())
        })
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
    let known: BTreeSet<&'static str> = unlock_flag_groups()
        .into_iter()
        .map(|group| group.flag)
        .collect();
    let selected = names
        .into_iter()
        .filter(|name| known.contains(name.as_str()))
        .collect::<Vec<_>>();
    let next = normalize_unlock_flags(selected);
    if next.is_empty() {
        return "No Noita unlock flag files recognized. Drop save00/persistent/flags or select that folder."
            .to_string();
    }
    let count = next.len();
    unlock_flags.set(next);
    format!("Imported {count} unlock flags from save files.")
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
    let groups = unlock_flag_groups();
    let total = groups.len();
    let selected_count = move || unlock_flags.with(Vec::len);
    let all_flags = move |_| {
        let next = all_unlock_flags();
        unlock_flags.set(next);
        import_status.set(format!("Enabled all {total} unlock flags."));
    };
    let no_flags = move |_| {
        let next = Vec::new();
        unlock_flags.set(next);
        import_status.set("Disabled every unlock flag.".to_string());
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
                    <span>
                        <span class="panel-title">"Unlock flags"</span>
                        <small>"Save-aware spell pools"</small>
                    </span>
                    <b>{move || format!("{}/{}", selected_count(), total)}</b>
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
                    <div class="unlock-actions">
                        <button type="button" class="btn btn-ghost" on:click=all_flags>"all unlocked"</button>
                        <button type="button" class="btn btn-ghost" on:click=no_flags>"no unlocks"</button>
                    </div>
                    <div class="unlock-flag-list">
                        <For each=move || groups.clone() key=|group| group.flag let:group>
                            <label class="unlock-flag-row">
                                <input
                                    type="checkbox"
                                    prop:checked=move || unlock_flags.with(|flags| flags.iter().any(|item| item == group.flag))
                                    on:change=move |ev| set_flag(unlock_flags, group.flag, event_target_checked(&ev))
                                />
                                <span class="unlock-flag-copy">
                                    <b>{group.flag}</b>
                                    <small>{group.spells.join(" · ")}</small>
                                </span>
                            </label>
                        </For>
                    </div>
                </div>
            </details>
        </fieldset>
    }
}
