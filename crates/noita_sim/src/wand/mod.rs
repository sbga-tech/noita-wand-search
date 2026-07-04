mod deck;
mod generator;
#[cfg(feature = "profiling")]
mod profiling;
mod sprites;
mod stats;

use crate::data::Spell;
use crate::rng::NollaPrng;
use crate::types::{SaveFlags, Wand};

use deck::add_random_cards;
pub use generator::WandGenerator;
#[cfg(feature = "profiling")]
pub use profiling::{profile_get_wand_unlocked, WandGenerationPhaseProfile};
use sprites::{consume_sprite_rng, select_best_sprite};
use stats::get_wand_stats;

pub fn get_wand_sprite(
    seed: u32,
    x: f64,
    y: f64,
    cost: i32,
    level: i32,
    force_unshuffle: bool,
) -> usize {
    let mut random = NollaPrng::new(seed);
    random.set_random_seed(x, y);
    let wand = get_wand_stats(cost, level, force_unshuffle, &mut random);
    select_best_sprite(&mut random, &wand)
}

pub fn get_wand_unlocked(
    seed: u32,
    x: f64,
    y: f64,
    cost: i32,
    level: i32,
    force_unshuffle: bool,
    save_flags: Option<&SaveFlags>,
) -> Wand {
    let mut random = NollaPrng::new(seed);
    random.set_random_seed(x, y);
    let mut wand = get_wand_stats(cost, level, force_unshuffle, &mut random);
    consume_sprite_rng(&mut random, &wand);
    wand.spells.clear();
    add_random_cards(&mut wand, seed, x, y, level, &mut random, save_flags);
    let capacity = wand.capacity.floor().max(0.0) as usize;
    wand.spells
        .resize(capacity.max(wand.spells.len()), Spell::None);
    wand.into_public()
}
