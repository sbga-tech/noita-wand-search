use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::filters::{Comparison, WandFilter, WandFilterSet};
use crate::loot::{
    Item, ItemFlags, LootSpawner, SpawnCoord, GREAT_CHEST_LOOT_TABLE, TAIKASAUVA_LOOT_TABLE,
    TINY_DROP_LOOT_TABLE,
};
use crate::search::{SearchMode, SearchRequest, SearchState};
use crate::types::{SaveFlags, WandStat};
use crate::wand::{get_wand_unlocked, profile_get_wand_unlocked, WandGenerationPhaseProfile};

#[derive(Clone, Copy, Debug)]
enum ProfileMode {
    Eoe,
    Taikasauva,
    TinyDrop,
}

#[derive(Clone, Debug)]
struct Config {
    mode: ProfileMode,
    seed: u32,
    ng: u32,
    start_x: f64,
    start_y: f64,
    iterations: usize,
    batch: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: ProfileMode::Eoe,
            seed: 123_456,
            ng: 0,
            start_x: 0.0,
            start_y: 0.0,
            iterations: 1 << 20,
            batch: 1 << 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ItemCounts {
    wands: usize,
    materials: usize,
    placeholders: usize,
}

impl ItemCounts {
    fn record(&mut self, item: &Item) {
        match item {
            Item::Wand(_) => self.wands += 1,
            Item::Material(_) => self.materials += 1,
            Item::Placeholder(_) => self.placeholders += 1,
        }
    }

    fn total(self) -> usize {
        self.wands + self.materials + self.placeholders
    }
}

struct Timed<T> {
    label: &'static str,
    elapsed: Duration,
    units: usize,
    value: T,
}

impl<T> Timed<T> {
    fn print(self, unit_label: &str, format_value: impl FnOnce(T) -> String) {
        let secs = self.elapsed.as_secs_f64();
        let units_per_sec = self.units as f64 / secs;
        let ns_per_unit = self.elapsed.as_nanos() as f64 / self.units.max(1) as f64;
        println!(
            "{:<28} {:>10.3} ms  {:>14.2} {}/s  {:>10.2} ns/{}  {}",
            self.label,
            secs * 1_000.0,
            units_per_sec,
            unit_label,
            ns_per_unit,
            unit_label,
            format_value(self.value),
        );
    }
}

pub fn main() {
    let config = parse_config(env::args().skip(1));
    println!("profile config: {config:?}");
    println!();

    let coords = sample_coords(&config);
    bench_search(&config).print("pixel", |searched| format!("searched={searched}"));
    bench_loot_iter(&config, &coords, ItemFlags::WAND, "loot wand projection")
        .print("coord", format_item_counts);
    bench_loot_iter(&config, &coords, ItemFlags::ALL, "loot full iterator")
        .print("coord", format_item_counts);
    bench_direct_wands(&config, &coords).print("wand", |count| format!("generated={count}"));
    print_wand_phase_profile(bench_profiled_wand_phases(&config, &coords));
}

fn parse_config(args: impl Iterator<Item = String>) -> Config {
    let mut config = Config::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mode" => {
                let value = next_arg(&mut args, "--mode");
                config.mode = match value.as_str() {
                    "eoe" => ProfileMode::Eoe,
                    "taikasauva" => ProfileMode::Taikasauva,
                    "tiny" | "tinydrop" => ProfileMode::TinyDrop,
                    _ => panic!("unknown --mode {value}; expected eoe, taikasauva, or tiny"),
                };
            }
            "--seed" => config.seed = parse_next(&mut args, "--seed"),
            "--ng" => config.ng = parse_next(&mut args, "--ng"),
            "--x" => config.start_x = parse_next(&mut args, "--x"),
            "--y" => config.start_y = parse_next(&mut args, "--y"),
            "--iterations" => config.iterations = parse_next(&mut args, "--iterations"),
            "--batch" => config.batch = parse_next(&mut args, "--batch"),
            "--help" | "-h" => print_help_and_exit(),
            _ => panic!("unknown profiling argument {arg}; pass --help for usage"),
        }
    }

    config
}

fn next_arg(args: &mut impl Iterator<Item = String>, name: &str) -> String {
    args.next()
        .unwrap_or_else(|| panic!("missing value after {name}"))
}

fn parse_next<T: std::str::FromStr>(args: &mut impl Iterator<Item = String>, name: &str) -> T
where
    T::Err: std::fmt::Display,
{
    let value = next_arg(args, name);
    value
        .parse()
        .unwrap_or_else(|err| panic!("invalid {name} value {value:?}: {err}"))
}

fn print_help_and_exit() -> ! {
    println!(
        "usage: cargo run --release -p noita_sim --features profiling --bin profile_search -- \\\n         [--mode eoe|taikasauva|tiny] [--seed N] [--ng N] [--x F] [--y F] \\\n         [--iterations N] [--batch N]"
    );
    std::process::exit(0);
}

fn sample_coords(config: &Config) -> Vec<SpawnCoord> {
    let x_base = config.start_x as i32;
    let y_base = config.start_y as i32;
    (0..config.iterations)
        .map(|index| SpawnCoord {
            x: x_base.wrapping_add(index as i32).wrapping_mul(37),
            y: y_base.wrapping_add(index as i32).wrapping_mul(53),
        })
        .collect()
}

fn impossible_filter() -> WandFilterSet {
    WandFilterSet {
        filters: vec![WandFilter::stat(
            WandStat::Capacity,
            Comparison::GreaterThan,
            1_000_000.0,
        )],
    }
}

fn search_mode(mode: ProfileMode) -> SearchMode {
    match mode {
        ProfileMode::Eoe => SearchMode::EoeWand,
        ProfileMode::Taikasauva => SearchMode::TaikasauvaWand,
        ProfileMode::TinyDrop => SearchMode::TinyDropWand,
    }
}

fn save_flags(mode: ProfileMode) -> Option<SaveFlags> {
    match mode {
        ProfileMode::Eoe => Some(SaveFlags::new(Vec::new())),
        ProfileMode::Taikasauva | ProfileMode::TinyDrop => None,
    }
}

fn loot_spawner(config: &Config) -> LootSpawner {
    let table = match config.mode {
        ProfileMode::Eoe => &GREAT_CHEST_LOOT_TABLE,
        ProfileMode::Taikasauva => &TAIKASAUVA_LOOT_TABLE,
        ProfileMode::TinyDrop => &TINY_DROP_LOOT_TABLE,
    };
    LootSpawner::new(
        config.seed.wrapping_add(config.ng),
        table,
        save_flags(config.mode),
    )
}

fn bench_search(config: &Config) -> Timed<u32> {
    let request = SearchRequest {
        seed: config.seed,
        ng: config.ng,
        start_x: config.start_x,
        start_y: config.start_y,
        mode: search_mode(config.mode),
        wand_filters: impossible_filter(),
        unlock_flags: None,
    };
    let mut state = SearchState::new(request);
    let start = Instant::now();
    while state.progress().searched_pixels < config.iterations as u32 {
        black_box(state.step(config.batch));
    }
    let elapsed = start.elapsed();
    let searched = state.progress().searched_pixels;
    Timed {
        label: "search step",
        elapsed,
        units: searched as usize,
        value: searched,
    }
}

fn bench_loot_iter(
    config: &Config,
    coords: &[SpawnCoord],
    item_flags: ItemFlags,
    label: &'static str,
) -> Timed<ItemCounts> {
    let spawner = loot_spawner(config);
    let mut counts = ItemCounts::default();
    let start = Instant::now();
    for &coord in coords {
        for item in spawner.iter_with_flags(coord, item_flags) {
            counts.record(black_box(&item));
        }
    }
    let elapsed = start.elapsed();
    Timed {
        label,
        elapsed,
        units: coords.len(),
        value: counts,
    }
}

fn generator_params(config: &Config) -> (i32, i32, bool, f64, f64) {
    match config.mode {
        ProfileMode::Eoe => (100, 5, false, 0.0, 0.0),
        ProfileMode::Taikasauva => (60, 3, false, 0.0, 0.0),
        ProfileMode::TinyDrop => (180, 11, true, 16.0, 0.0),
    }
}

fn bench_direct_wands(config: &Config, coords: &[SpawnCoord]) -> Timed<usize> {
    let (cost, level, force_nonshuffle, x_offset, y_offset) = generator_params(config);
    let save_flags = save_flags(config.mode);
    let start = Instant::now();
    for &coord in coords {
        black_box(get_wand_unlocked(
            config.seed.wrapping_add(config.ng),
            coord.x as f64 + x_offset,
            coord.y as f64 + y_offset,
            cost,
            level,
            force_nonshuffle,
            save_flags.as_ref(),
        ));
    }
    let elapsed = start.elapsed();
    Timed {
        label: "direct wand generation",
        elapsed,
        units: coords.len(),
        value: coords.len(),
    }
}

fn bench_profiled_wand_phases(
    config: &Config,
    coords: &[SpawnCoord],
) -> Timed<WandGenerationPhaseProfile> {
    let (cost, level, force_nonshuffle, x_offset, y_offset) = generator_params(config);
    let save_flags = save_flags(config.mode);
    let mut profile = WandGenerationPhaseProfile::default();
    let start = Instant::now();
    for &coord in coords {
        black_box(profile_get_wand_unlocked(
            config.seed.wrapping_add(config.ng),
            coord.x as f64 + x_offset,
            coord.y as f64 + y_offset,
            cost,
            level,
            force_nonshuffle,
            save_flags.as_ref(),
            &mut profile,
        ));
    }
    let elapsed = start.elapsed();
    Timed {
        label: "profiled wand phases",
        elapsed,
        units: coords.len(),
        value: profile,
    }
}

fn print_wand_phase_profile(timed: Timed<WandGenerationPhaseProfile>) {
    let profile = timed.value;
    let secs = timed.elapsed.as_secs_f64();
    let units_per_sec = timed.units as f64 / secs;
    let ns_per_unit = timed.elapsed.as_nanos() as f64 / timed.units.max(1) as f64;
    println!(
        "{:<28} {:>10.3} ms  {:>14.2} wand/s  {:>10.2} ns/wand  profiled={}",
        timed.label,
        secs * 1_000.0,
        units_per_sec,
        ns_per_unit,
        profile.wands,
    );

    let accounted =
        profile.setup + profile.stats + profile.sprite + profile.cards + profile.finalize;
    println!(
        "  phases: setup={} stats={} sprite={} cards={} finalize={} accounted={} internal_total={}",
        format_duration_pct(profile.setup, accounted),
        format_duration_pct(profile.stats, accounted),
        format_duration_pct(profile.sprite, accounted),
        format_duration_pct(profile.cards, accounted),
        format_duration_pct(profile.finalize, accounted),
        format_duration(accounted),
        format_duration(profile.total),
    );
    println!(
        "  sprite zero: wands={} ({:.2}%) matches={} rng_draws={}",
        profile.sprite_zero_match_wands,
        profile.sprite_zero_match_wands as f64 * 100.0 / profile.wands.max(1) as f64,
        profile.sprite_zero_matches,
        profile.sprite_zero_rng_draws,
    );
}

fn format_duration_pct(duration: Duration, total: Duration) -> String {
    let pct = if total.is_zero() {
        0.0
    } else {
        duration.as_secs_f64() * 100.0 / total.as_secs_f64()
    };
    format!("{} ({pct:.1}%)", format_duration(duration))
}

fn format_duration(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1_000.0)
}

fn format_item_counts(counts: ItemCounts) -> String {
    format!(
        "items={} wands={} materials={} placeholders={}",
        counts.total(),
        counts.wands,
        counts.materials,
        counts.placeholders,
    )
}
