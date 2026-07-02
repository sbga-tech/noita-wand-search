//! Copies the Noita image assets needed by the wand cards out of the
//! `noita-data` submodule and into `public/assets/` so Trunk can copy
//! them into the served site root. Only the three directories the UI reads are
//! mirrored, and files are copied only when missing or changed so incremental
//! builds stay cheap.

use std::path::{Path, PathBuf};
use std::{env, fs};

/// Subdirectories of the Noita data tree that the UI references, mapped to the
/// destination under `public/assets/`.
const ASSET_DIRS: &[(&str, &str)] = &[
    ("ui_gfx/gun_actions", "gun_actions"),
    ("ui_gfx/inventory", "inventory"),
    ("items_gfx/wands", "wands"),
];

fn main() {
    let source = resolve_data_source();
    println!("cargo:rerun-if-env-changed=NOITA_DATA_PATH");

    let dest_root = PathBuf::from("public/assets");
    for (inner, dest) in ASSET_DIRS {
        let src_dir = source.join(inner);
        assert!(
            src_dir.is_dir(),
            "Noita asset directory not found: {}",
            src_dir.display()
        );
        println!("cargo:rerun-if-changed={}", src_dir.display());
        mirror_pngs(&src_dir, &dest_root.join(dest));
    }
}

/// Resolves the Noita data root, mirroring `noita_sim`'s convention:
/// `NOITA_DATA_PATH`, then the submodule, then a locally extracted copy.
fn resolve_data_source() -> PathBuf {
    if let Some(path) = env::var_os("NOITA_DATA_PATH") {
        return PathBuf::from(path);
    }
    let submodule = PathBuf::from("../../noita-data");
    if submodule.is_dir() {
        return submodule;
    }
    let local = PathBuf::from("../noita_sim/data");
    assert!(
        local.is_dir(),
        "Noita data not found: set NOITA_DATA_PATH or init the noita-data submodule"
    );
    local
}

/// Copies every `*.png` from `src` into `dest`, creating `dest` and skipping
/// files whose destination already matches the source length.
fn mirror_pngs(src: &Path, dest: &Path) {
    fs::create_dir_all(dest)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", dest.display()));
    let entries =
        fs::read_dir(src).unwrap_or_else(|err| panic!("failed to read {}: {err}", src.display()));
    for entry in entries {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("png") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let target = dest.join(name);
        if is_up_to_date(&path, &target) {
            continue;
        }
        fs::copy(&path, &target).unwrap_or_else(|err| {
            panic!(
                "failed to copy {} -> {}: {err}",
                path.display(),
                target.display()
            )
        });
    }
}

/// True when `target` exists and has the same byte length as `src`.
fn is_up_to_date(src: &Path, target: &Path) -> bool {
    let (Ok(src_meta), Ok(target_meta)) = (fs::metadata(src), fs::metadata(target)) else {
        return false;
    };
    src_meta.len() == target_meta.len()
}
