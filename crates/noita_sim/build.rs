use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::Deserialize;
use serde_luaq::{from_slice, LuaFormat};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ACTION_TYPES: &[&str] = &[
    "PROJECTILE",
    "STATIC_PROJECTILE",
    "MODIFIER",
    "DRAW_MANY",
    "MATERIAL",
    "OTHER",
    "UTILITY",
    "PASSIVE",
];
const LEVEL_COUNT: usize = 11;
const TYPE_COUNT: usize = ACTION_TYPES.len();
const NOITA_DATA_DIR: &str = "../../noita-data";
const LOCAL_DATA_DIR: &str = "data";
const GENERATED_DATA_DIR: &str = "src/data/generated";
const GUN_ACTIONS: &str = "scripts/gun/gun_actions.lua";
const MATERIALS_XML: &str = "materials.xml";
const POTION_LUA: &str = "scripts/items/potion.lua";
const POTION_SECRET_LUA: &str = "scripts/items/potion_secret.lua";
const WANDS_LUA: &str = "scripts/gun/procedural/wands.lua";
const TRANSLATIONS_COMMON: &str = "translations/common.csv";
const ROOT_COMMON: &str = "common.csv";
const TRANSLATION_PATHS: &[&str] = &[TRANSLATIONS_COMMON, ROOT_COMMON];
const DATA_SOURCE_FILES: &[&str] = &[
    GUN_ACTIONS,
    MATERIALS_XML,
    POTION_LUA,
    POTION_SECRET_LUA,
    WANDS_LUA,
    TRANSLATIONS_COMMON,
    ROOT_COMMON,
];

#[derive(Clone, Debug)]
struct Translations {
    locales: Vec<String>,
    values: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct Action {
    id: String,
    variant: String,
    display_names: Vec<String>,
    action_type_index: usize,
    spawn_levels: Vec<usize>,
    spawn_probabilities: Vec<f32>,
    unlock_flag: Option<String>,
    icon: String,
}

fn main() {
    let default_submodule_source = PathBuf::from(NOITA_DATA_DIR);
    let local_source = PathBuf::from(LOCAL_DATA_DIR);
    let source = env::var_os("NOITA_DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if default_submodule_source.is_dir() {
                default_submodule_source
            } else {
                local_source
            }
        });
    assert!(
        source.is_dir(),
        "Noita data not found: {}",
        source.display()
    );
    let generated_dir = PathBuf::from(GENERATED_DATA_DIR);

    println!("cargo:rerun-if-env-changed=NOITA_DATA_PATH");
    println!("cargo:rerun-if-changed={}", source.display());
    for inner in DATA_SOURCE_FILES {
        println!("cargo:rerun-if-changed={}", source.join(inner).display());
    }

    let translations = parse_translations(&source);
    let lua = read_source_text(&source, GUN_ACTIONS);
    let actions = parse_actions(&lua, &translations);
    assert!(
        actions.len() > 1,
        "no spells parsed from {}",
        source.display()
    );

    fs::create_dir_all(&generated_dir).unwrap_or_else(|err| {
        panic!(
            "failed to create generated source directory {}: {err}",
            generated_dir.display()
        )
    });
    write_if_changed(
        generated_dir.join("spells.rs"),
        &generated_spells(&actions, &translations.locales),
    );
    write_if_changed(
        generated_dir.join("spell_probs.rs"),
        &generated_spell_probs(&actions),
    );
    write_if_changed(
        generated_dir.join("materials.rs"),
        &generated_materials(&source),
    );
    write_if_changed(
        generated_dir.join("wand_sprites.rs"),
        &generated_wand_sprites(&source),
    );
}

fn write_if_changed(path: PathBuf, content: &str) {
    if fs::read_to_string(&path).ok().as_deref() == Some(content) {
        return;
    }
    fs::write(&path, content)
        .unwrap_or_else(|err| panic!("failed to write generated source {}: {err}", path.display()));
}

fn format_generated_file(tokens: TokenStream) -> String {
    let source = tokens.to_string();
    let file = syn::parse_file(&source)
        .unwrap_or_else(|err| panic!("generated Rust did not parse: {err}\n{source}"));
    prettyplease::unparse(&file)
}

fn read_source_text(source: &Path, inner: &str) -> String {
    let path = source.join(inner);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn read_first_source_text(source: &Path, inners: &[&str]) -> (String, String) {
    for inner in inners {
        let path = source.join(inner);
        if path.is_file() {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            return (text, (*inner).to_string());
        }
    }
    panic!(
        "missing data file. Tried [{}] in {}",
        inners.join(", "),
        source.display()
    );
}

fn parse_translations(source: &Path) -> Translations {
    let (text, path) = read_first_source_text(source, TRANSLATION_PATHS);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut records = reader.records();
    let header = records
        .next()
        .unwrap_or_else(|| panic!("{path} is empty"))
        .unwrap_or_else(|err| panic!("failed to parse {path} header: {err}"));
    let locales = header
        .iter()
        .skip(1)
        .take_while(|column| !column.is_empty() && !column.starts_with("NOTES"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert!(
        locales.iter().any(|locale| locale == "en"),
        "{path} does not contain an en column"
    );
    let mut values = HashMap::new();
    for record in records {
        let record = record.unwrap_or_else(|err| panic!("failed to parse {path}: {err}"));
        let Some(key) = record.get(0).filter(|key| !key.is_empty()) else {
            continue;
        };
        let english = locales
            .iter()
            .position(|locale| locale == "en")
            .and_then(|index| record.get(index + 1))
            .unwrap_or("");
        let row = locales
            .iter()
            .enumerate()
            .map(|(index, _)| {
                record
                    .get(index + 1)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(english)
                    .replace("\\n", "\n")
            })
            .collect::<Vec<_>>();
        values.insert(key.to_string(), row);
    }
    Translations { locales, values }
}

const LUA_MAX_TABLE_DEPTH: u16 = 32;

#[derive(Deserialize)]
#[serde(untagged)]
enum LuaFieldValue {
    String(String),
    Ignored(IgnoredAny),
}

type LuaFields = HashMap<String, LuaFieldValue>;

#[derive(Clone, Debug, Deserialize)]
struct RawMaterialEntry {
    material: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawWandSprite {
    name: String,
    file: String,
    grip_x: i8,
    grip_y: i8,
    tip_x: i8,
    tip_y: i8,
    fire_rate_wait: i8,
    actions_per_round: i8,
    shuffle_deck_when_empty: i8,
    deck_capacity: i8,
    spread_degrees: i8,
    reload_time: i8,
}

fn action_type_by_lua(lua_name: &str) -> Option<usize> {
    ACTION_TYPES
        .iter()
        .position(|candidate| *candidate == lua_name)
}

fn split_words(identifier: &str) -> impl Iterator<Item = &str> {
    identifier.split('_').filter(|word| !word.is_empty())
}

fn variant_name(identifier: &str) -> String {
    let mut variant = String::new();
    for word in split_words(identifier) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            variant.extend(first.to_uppercase());
            variant.push_str(&chars.as_str().to_lowercase());
        }
    }
    if variant.is_empty() {
        "Unknown".to_string()
    } else if variant.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("N{variant}")
    } else {
        variant
    }
}

/// Extracts the icon basename (without directory or `.png`) from a lua `sprite`
/// path such as `data/ui_gfx/gun_actions/mana.png` -> `mana`.
fn sprite_icon_name(sprite: &str) -> String {
    sprite
        .rsplit('/')
        .next()
        .unwrap_or(sprite)
        .strip_suffix(".png")
        .unwrap_or(sprite)
        .to_string()
}

fn strip_lua_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < source.len() {
        let rest = &source[index..];
        let ch = rest.chars().next().expect("valid char boundary");
        if in_string {
            out.push(ch);
            index += ch.len_utf8();
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            index += ch.len_utf8();
            continue;
        }
        if rest.starts_with("--[[") {
            let after_open = index + 4;
            if let Some(close_offset) = source[after_open..].find("]]") {
                let block = &source[after_open..after_open + close_offset];
                let trimmed = block.trim_start();
                let after_table_open = trimmed
                    .strip_prefix('{')
                    .map(str::trim_start)
                    .unwrap_or_default();
                let looks_like_table = trimmed.starts_with('{')
                    && (after_table_open.starts_with('{')
                        || after_table_open.starts_with("id")
                        || after_table_open.starts_with("material"));
                if looks_like_table {
                    out.push_str(&strip_lua_comments(block));
                } else {
                    out.push('\n');
                }
                index = after_open + close_offset + 2;
                if source[index..].starts_with("--") {
                    index += 2;
                }
            } else {
                break;
            }
            continue;
        }
        if rest.starts_with("--") {
            if let Some(newline_offset) = rest.find('\n') {
                out.push('\n');
                index += newline_offset + 1;
            } else {
                break;
            }
            continue;
        }
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn is_lua_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_lua_word_at(source: &str, index: usize, word: &str) -> bool {
    let bytes = source.as_bytes();
    let end = index + word.len();
    source[index..].starts_with(word)
        && (index == 0 || !is_lua_identifier_byte(bytes[index - 1]))
        && (end == bytes.len() || !is_lua_identifier_byte(bytes[end]))
}

fn line_indent(source: &str, index: usize) -> &str {
    let line_start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
    let indent_end = source[line_start..]
        .char_indices()
        .find_map(|(offset, ch)| (!ch.is_ascii_whitespace()).then_some(line_start + offset))
        .unwrap_or(index);
    &source[line_start..indent_end]
}

fn skip_line_whitespace(source: &str, mut index: usize) -> usize {
    let bytes = source.as_bytes();
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\r') {
        index += 1;
    }
    index
}

fn find_lua_function_end(source: &str, start: usize) -> Option<usize> {
    let indent = line_indent(source, start);
    let mut search = start + "function".len();
    while let Some(relative_newline) = source[search..].find('\n') {
        let line_start = search + relative_newline + 1;
        if source[line_start..].starts_with(indent) {
            let candidate = line_start + indent.len();
            if is_lua_word_at(source, candidate, "end") {
                let after_end = candidate + "end".len();
                let after_space = skip_line_whitespace(source, after_end);
                if after_space == source.len()
                    || matches!(source.as_bytes().get(after_space), Some(b',') | Some(b'\n'))
                {
                    return Some(after_end);
                }
            }
        }
        search = line_start;
    }
    None
}

fn replace_lua_functions(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < source.len() {
        let rest = &source[index..];
        let ch = rest.chars().next().expect("valid char boundary");
        if in_string {
            out.push(ch);
            index += ch.len_utf8();
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            index += ch.len_utf8();
            continue;
        }
        if is_lua_word_at(source, index, "function") {
            let after_function = find_lua_function_end(source, index)
                .unwrap_or_else(|| panic!("failed to find end for Lua function near byte {index}"));
            let is_assignment_value = out.chars().rev().find(|ch| !ch.is_whitespace()) == Some('=');
            if is_assignment_value {
                out.push_str("nil");
            } else {
                out.push('\n');
            }
            index = after_function;
            continue;
        }
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn remove_lua_call_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("dofile_once(") || trimmed.starts_with("dofile(") {
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn quote_lua_action_type_identifiers(source: &str) -> String {
    let mut out = source.to_string();
    for action_type in ACTION_TYPES {
        out = out.replace(
            &format!("ACTION_TYPE_{action_type}"),
            &format!("\"{action_type}\""),
        );
    }
    out
}

fn normalize_lua_script(source: &str) -> String {
    let without_comments = strip_lua_comments(source);
    let without_functions = replace_lua_functions(&without_comments);
    let without_calls = remove_lua_call_lines(&without_functions);
    quote_lua_action_type_identifiers(&without_calls)
}

fn lua_table<T>(source: &str, table_name: &str) -> Vec<T>
where
    T: DeserializeOwned,
{
    let normalized = normalize_lua_script(source);
    let mut tables: HashMap<String, Vec<T>> = from_slice(
        normalized.as_bytes(),
        LuaFormat::Script,
        LUA_MAX_TABLE_DEPTH,
    )
    .unwrap_or_else(|err| panic!("failed to parse Lua tables for {table_name}: {err}"));
    tables.remove(table_name).unwrap_or_default()
}

fn lua_table_fields(source: &str, table_name: &str) -> Vec<LuaFields> {
    lua_table::<LuaFields>(source, table_name)
}

fn field_string(entry: &LuaFields, field: &str) -> Option<String> {
    match entry.get(field)? {
        LuaFieldValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn parse_int_list(value: Option<String>) -> Vec<usize> {
    value
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .collect()
}

fn parse_float_list(value: Option<String>) -> Vec<f32> {
    value
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect()
}

fn translated_names(translations: &Translations, raw_key: &str, action_id: &str) -> Vec<String> {
    let Some(key) = raw_key.strip_prefix('$') else {
        return vec![raw_key.to_string(); translations.locales.len()];
    };
    translations.values.get(key).cloned().unwrap_or_else(|| {
        panic!("missing English translation for spell {action_id} name key {key}")
    })
}

fn parse_actions(source: &str, translations: &Translations) -> Vec<Action> {
    let mut actions = vec![Action {
        id: "NONE".to_string(),
        variant: "None".to_string(),
        display_names: vec!["None".to_string(); translations.locales.len()],
        action_type_index: 0,
        spawn_levels: Vec::new(),
        spawn_probabilities: Vec::new(),
        unlock_flag: None,
        icon: String::new(),
    }];
    let mut seen_ids = HashSet::from(["NONE".to_string()]);
    for entry in lua_table_fields(source, "actions") {
        let Some(id) = field_string(&entry, "id") else {
            continue;
        };
        let Some(lua_type) = field_string(&entry, "type") else {
            continue;
        };
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        let name_key = field_string(&entry, "name")
            .unwrap_or_else(|| panic!("spell {id} is missing translation name key"));
        let action_type_index = action_type_by_lua(&lua_type)
            .unwrap_or_else(|| panic!("unknown action type ACTION_TYPE_{lua_type} for {id}"));
        let icon = field_string(&entry, "sprite")
            .as_deref()
            .map(sprite_icon_name)
            .unwrap_or_default();
        actions.push(Action {
            variant: variant_name(&id),
            display_names: translated_names(translations, &name_key, &id),
            action_type_index,
            spawn_levels: parse_int_list(field_string(&entry, "spawn_level")),
            spawn_probabilities: parse_float_list(field_string(&entry, "spawn_probability")),
            unlock_flag: field_string(&entry, "spawn_requires_flag"),
            icon,
            id,
        });
    }
    actions
}

#[derive(Clone, Debug)]
struct MaterialSource {
    variant: String,
    canonical_name: String,
    tags: String,
}

fn parse_materials(source: &Path) -> Vec<MaterialSource> {
    let text = read_source_text(source, MATERIALS_XML);
    let document = roxmltree::Document::parse(&text)
        .unwrap_or_else(|err| panic!("failed to parse {MATERIALS_XML}: {err}"));
    let mut materials = Vec::new();
    let mut seen_names = HashSet::new();
    let mut seen_variants = HashSet::new();
    for node in document
        .descendants()
        .filter(|node| node.has_tag_name("CellData") || node.has_tag_name("CellDataChild"))
    {
        let Some(name) = node.attribute("name") else {
            continue;
        };
        let variant = variant_name(name);
        if !seen_names.insert(name.to_string()) || !seen_variants.insert(variant.clone()) {
            continue;
        }
        materials.push(MaterialSource {
            variant,
            canonical_name: name.to_string(),
            tags: node.attribute("tags").unwrap_or_default().to_string(),
        });
    }
    assert!(
        !materials.is_empty(),
        "{MATERIALS_XML} did not contain materials"
    );
    materials
}

fn lua_material_list(source: &Path, inner: &str, table_name: &str) -> Vec<String> {
    let text = read_source_text(source, inner);
    lua_table::<RawMaterialEntry>(&text, table_name)
        .into_iter()
        .filter_map(|entry| entry.material)
        .collect()
}

fn material_variant<'a>(materials: &'a [MaterialSource], canonical_name: &str) -> &'a str {
    materials
        .iter()
        .find(|material| material.canonical_name == canonical_name)
        .unwrap_or_else(|| panic!("material {canonical_name} not found in {MATERIALS_XML}"))
        .variant
        .as_str()
}

fn generated_materials(source: &Path) -> String {
    let materials = parse_materials(source);
    let standard = lua_material_list(source, POTION_LUA, "materials_standard");
    let magic = lua_material_list(source, POTION_LUA, "materials_magic");
    let secret = lua_material_list(source, POTION_SECRET_LUA, "potions");
    let sands = materials
        .iter()
        .filter(|material| material.tags.contains("[sand"))
        .map(|material| material.canonical_name.clone())
        .collect::<Vec<_>>();
    let liquids = materials
        .iter()
        .filter(|material| material.tags.contains("[liquid]"))
        .map(|material| material.canonical_name.clone())
        .collect::<Vec<_>>();

    let variants = materials
        .iter()
        .map(|material| format_ident!("{}", material.variant))
        .collect::<Vec<_>>();
    let canonical_names = materials
        .iter()
        .map(|material| Literal::string(&material.canonical_name))
        .collect::<Vec<_>>();
    let canonical_names_ci = materials
        .iter()
        .map(|material| Literal::string(&material.canonical_name.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let material_count = materials.len();
    let pool_consts = [
        ("POTION_MATERIALS_STANDARD", standard),
        ("POTION_MATERIALS_MAGIC", magic),
        ("POTION_MATERIALS_SECRET", secret),
        ("POTION_SANDS", sands),
        ("POTION_LIQUIDS", liquids),
    ]
    .into_iter()
    .map(|(name, pool)| {
        let const_name = format_ident!("{name}");
        let entries = pool.iter().map(|canonical_name| {
            let variant = format_ident!("{}", material_variant(&materials, canonical_name));
            quote! { Material::#variant }
        });
        quote! {
            pub const #const_name: &[Material] = &[#(#entries),*];
        }
    });

    format_generated_file(quote! {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            num_enum::IntoPrimitive,
            num_enum::TryFromPrimitive,
        )]
        #[repr(u16)]
        pub enum Material {
            #(#variants,)*
        }

        impl Material {
            pub const COUNT: usize = #material_count;

            pub fn canonical_name(self) -> &'static str {
                match self {
                    #(Self::#variants => #canonical_names,)*
                }
            }

            pub fn from_canonical_name(value: &str) -> Option<Self> {
                let value = value.to_ascii_lowercase();
                match value.as_str() {
                    #(#canonical_names_ci => Some(Self::#variants),)*
                    _ => None,
                }
            }
        }

        #(#pool_consts)*
    })
}

#[derive(Clone, Debug)]
struct WandSpriteSource {
    name: String,
    file_num: i32,
    grip_x: i8,
    grip_y: i8,
    tip_x: i8,
    tip_y: i8,
    fire_rate_wait: i8,
    actions_per_round: i8,
    shuffle_deck_when_empty: bool,
    deck_capacity: i8,
    spread_degrees: i8,
    reload_time: i8,
}

fn wand_file_num(file: &str) -> Option<i32> {
    let start = file.rfind("wand_")? + "wand_".len();
    let digits = file[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

fn parse_wand_sprites(source: &Path) -> Vec<WandSpriteSource> {
    let text = read_source_text(source, WANDS_LUA);
    let mut seen_file_nums = HashSet::new();
    let mut rows = Vec::new();
    for entry in lua_table::<RawWandSprite>(&text, "wands") {
        let file_num = wand_file_num(&entry.file).unwrap_or_else(|| {
            panic!(
                "wand sprite file {} does not contain wand number",
                entry.file
            )
        });
        assert!(
            seen_file_nums.insert(file_num),
            "duplicate wand sprite file number {file_num}"
        );
        rows.push(WandSpriteSource {
            name: entry.name,
            file_num,
            grip_x: entry.grip_x,
            grip_y: entry.grip_y,
            tip_x: entry.tip_x,
            tip_y: entry.tip_y,
            fire_rate_wait: entry.fire_rate_wait,
            actions_per_round: entry.actions_per_round,
            shuffle_deck_when_empty: entry.shuffle_deck_when_empty != 0,
            deck_capacity: entry.deck_capacity,
            spread_degrees: entry.spread_degrees,
            reload_time: entry.reload_time,
        });
    }
    rows.sort_by_key(|row| row.file_num);
    assert_eq!(
        rows.len(),
        1000,
        "expected exactly 1000 wand sprites from {WANDS_LUA}"
    );
    assert!(
        rows.iter()
            .enumerate()
            .all(|(index, row)| row.file_num == index as i32),
        "wand sprite file numbers must be contiguous from 0 to 999"
    );
    rows
}

fn generated_wand_sprites(source: &Path) -> String {
    let rows = parse_wand_sprites(source);
    let rows = rows.iter().map(|row| {
        let name = Literal::string(&row.name);
        let file_num = Literal::i32_unsuffixed(row.file_num);
        let grip_x = Literal::i8_unsuffixed(row.grip_x);
        let grip_y = Literal::i8_unsuffixed(row.grip_y);
        let tip_x = Literal::i8_unsuffixed(row.tip_x);
        let tip_y = Literal::i8_unsuffixed(row.tip_y);
        let fire_rate_wait = Literal::i8_unsuffixed(row.fire_rate_wait);
        let actions_per_round = Literal::i8_unsuffixed(row.actions_per_round);
        let shuffle_deck_when_empty = row.shuffle_deck_when_empty;
        let deck_capacity = Literal::i8_unsuffixed(row.deck_capacity);
        let spread_degrees = Literal::i8_unsuffixed(row.spread_degrees);
        let reload_time = Literal::i8_unsuffixed(row.reload_time);
        quote! {
            WandSprite {
                name: #name,
                file_num: #file_num,
                grip_x: #grip_x,
                grip_y: #grip_y,
                tip_x: #tip_x,
                tip_y: #tip_y,
                fire_rate_wait: #fire_rate_wait,
                actions_per_round: #actions_per_round,
                shuffle_deck_when_empty: #shuffle_deck_when_empty,
                deck_capacity: #deck_capacity,
                spread_degrees: #spread_degrees,
                reload_time: #reload_time,
            }
        }
    });

    format_generated_file(quote! {
        use crate::data::WandSprite;

        pub static WAND_SPRITES: &[WandSprite] = &[#(#rows),*];
    })
}

fn generated_spells(actions: &[Action], locales: &[String]) -> String {
    let count = actions.len();
    let variants = actions
        .iter()
        .map(|action| format_ident!("{}", action.variant))
        .collect::<Vec<_>>();
    let ids = actions
        .iter()
        .map(|action| Literal::string(&action.id))
        .collect::<Vec<_>>();
    let icons = actions
        .iter()
        .map(|action| Literal::string(&action.icon))
        .collect::<Vec<_>>();
    let en_index = locales
        .iter()
        .position(|locale| locale == "en")
        .expect("translations must include en");
    let display_name_arms = locales
        .iter()
        .enumerate()
        .map(|(locale_index, locale)| {
            let locale = Literal::string(locale);
            let names = actions
                .iter()
                .map(|action| Literal::string(&action.display_names[locale_index]))
                .collect::<Vec<_>>();
            quote! {
                #locale => match self {
                    #(Self::#variants => #names,)*
                },
            }
        })
        .collect::<Vec<_>>();
    let fallback_display_names = actions
        .iter()
        .map(|action| Literal::string(&action.display_names[en_index]))
        .collect::<Vec<_>>();
    let lookup_arms = locales
        .iter()
        .enumerate()
        .map(|(locale_index, locale)| {
            let locale = Literal::string(locale);
            let mut seen_display_names = HashSet::new();
            let mut lookup_display_names = Vec::new();
            let mut lookup_variants = Vec::new();
            for action in actions {
                let key = &action.display_names[locale_index];
                if seen_display_names.insert(key.clone()) {
                    lookup_display_names.push(Literal::string(key));
                    lookup_variants.push(format_ident!("{}", action.variant));
                }
            }
            quote! {
                #locale => match value {
                    #(#lookup_display_names => Some(Self::#lookup_variants),)*
                    _ => None,
                },
            }
        })
        .collect::<Vec<_>>();
    let mut seen_display_names = HashSet::new();
    let mut fallback_lookup_display_names = Vec::new();
    let mut fallback_lookup_variants = Vec::new();
    for action in actions {
        let key = &action.display_names[en_index];
        if seen_display_names.insert(key.clone()) {
            fallback_lookup_display_names.push(Literal::string(key));
            fallback_lookup_variants.push(format_ident!("{}", action.variant));
        }
    }
    let unlock_flags = actions
        .iter()
        .map(|action| match &action.unlock_flag {
            Some(flag) => {
                let flag = Literal::string(flag);
                quote! { Some(#flag) }
            }
            None => quote! { None },
        })
        .collect::<Vec<_>>();

    format_generated_file(quote! {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            num_enum::IntoPrimitive,
            num_enum::TryFromPrimitive,
        )]
        #[repr(u16)]
        pub enum Spell {
            #(#variants,)*
        }

        impl Spell {
            pub const COUNT: usize = #count;
            pub const ALL: [Self; #count] = [#(Self::#variants),*];

            pub fn id(self) -> &'static str {
                match self {
                    #(Self::#variants => #ids,)*
                }
            }

            /// Icon basename under `ui_gfx/gun_actions/` (no directory, no
            /// `.png`). Empty for [`Spell::None`].
            pub fn icon(self) -> &'static str {
                match self {
                    #(Self::#variants => #icons,)*
                }
            }

            pub fn display_name(self, locale: &str) -> &'static str {
                match locale {
                    #(#display_name_arms)*
                    _ => match self {
                        #(Self::#variants => #fallback_display_names,)*
                    },
                }
            }

            pub fn unlock_flag(self) -> Option<&'static str> {
                match self {
                    #(Self::#variants => #unlock_flags,)*
                }
            }

            pub fn from_display_name(value: &str, locale: &str) -> Option<Self> {
                match locale {
                    #(#lookup_arms)*
                    _ => match value {
                        #(#fallback_lookup_display_names => Some(Self::#fallback_lookup_variants),)*
                        _ => None,
                    },
                }
            }
        }
    })
}

fn cumulative_entries(
    actions: &[Action],
    level: usize,
    type_index: Option<usize>,
) -> Vec<(f32, String)> {
    let mut total = 0.0f32;
    let mut rows = Vec::new();
    for action in &actions[1..] {
        if type_index.is_some_and(|index| index != action.action_type_index) {
            continue;
        }
        for (&spawn_level, &probability) in
            action.spawn_levels.iter().zip(&action.spawn_probabilities)
        {
            if spawn_level == level && probability > 0.0 {
                total += probability;
                rows.push((total, action.variant.clone()));
                break;
            }
        }
    }
    rows
}

fn prob_table(name: &str, rows: &[(f32, String)]) -> TokenStream {
    let name = format_ident!("{name}");
    let rows = rows.iter().map(|(total, variant)| {
        let total = format!("{total:.6}").parse::<TokenStream>().unwrap();
        let variant = format_ident!("{variant}");
        quote! {
            SpellProb {
                p: #total,
                spell: Spell::#variant,
            }
        }
    });
    quote! {
        pub const #name: &[SpellProb] = &[#(#rows),*];
    }
}

fn generated_spell_probs(actions: &[Action]) -> String {
    let level_tables: Vec<Vec<(f32, String)>> = (0..LEVEL_COUNT)
        .map(|level| cumulative_entries(actions, level, None))
        .collect();
    let type_tables: Vec<Vec<Vec<(f32, String)>>> = (0..LEVEL_COUNT)
        .map(|level| {
            (0..TYPE_COUNT)
                .map(|type_index| cumulative_entries(actions, level, Some(type_index)))
                .collect()
        })
        .collect();

    let level_consts = level_tables
        .iter()
        .enumerate()
        .map(|(level, rows)| prob_table(&format!("SPELL_PROBS_{level}"), rows));
    let type_consts = type_tables
        .iter()
        .enumerate()
        .flat_map(|(level, level_by_type)| {
            level_by_type
                .iter()
                .enumerate()
                .filter(|(_, rows)| !rows.is_empty())
                .map(move |(type_index, rows)| {
                    prob_table(&format!("SPELL_PROBS_{level}_T{type_index}"), rows)
                })
        });
    let all_spell_probs = (0..LEVEL_COUNT).map(|level| format_ident!("SPELL_PROBS_{level}"));
    let spell_prob_type_rows = type_tables
        .iter()
        .enumerate()
        .map(|(level, level_by_type)| {
            let entries = level_by_type.iter().enumerate().map(|(type_index, rows)| {
                if rows.is_empty() {
                    quote! { EMPTY }
                } else {
                    let table = format_ident!("SPELL_PROBS_{level}_T{type_index}");
                    quote! { #table }
                }
            });
            quote! { [#(#entries),*] }
        });
    let spell_prob_count_rows = type_tables.iter().map(|level_by_type| {
        let counts = level_by_type.iter().map(|rows| rows.len());
        quote! { [#(#counts),*] }
    });

    format_generated_file(quote! {
        use crate::data::{Spell, SpellProb};

        const EMPTY: &[SpellProb] = &[];

        #(#level_consts)*
        #(#type_consts)*

        pub const ALL_SPELL_PROBS: [&[SpellProb]; 11] = [#(#all_spell_probs),*];

        pub const SPELL_PROBS_TYPES: [[&[SpellProb]; 8]; 11] = [#(#spell_prob_type_rows),*];

        pub const SPELL_PROBS_COUNTS: [[usize; 8]; 11] = [#(#spell_prob_count_rows),*];
    })
}
