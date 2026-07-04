use super::deck::add_random_cards;
use super::sprites::consume_sprite_rng_profiled;
use super::stats::get_wand_stats;
use crate::data::Spell;
use crate::rng::NollaPrng;
use crate::types::{SaveFlags, Wand};

#[cfg(feature = "profiling")]
#[derive(Clone, Copy, Debug, Default)]
pub struct WandGenerationPhaseProfile {
    pub wands: usize,
    pub setup: std::time::Duration,
    pub stats: std::time::Duration,
    pub sprite: std::time::Duration,
    pub cards: std::time::Duration,
    pub finalize: std::time::Duration,
    pub total: std::time::Duration,
    pub sprite_zero_match_wands: usize,
    pub sprite_zero_matches: usize,
    pub sprite_zero_rng_draws: usize,
}

#[cfg(feature = "profiling")]
pub fn profile_get_wand_unlocked(
    seed: u32,
    x: f64,
    y: f64,
    cost: i32,
    level: i32,
    force_unshuffle: bool,
    save_flags: Option<&SaveFlags>,
    profile: &mut WandGenerationPhaseProfile,
) -> Wand {
    let total_start = std::time::Instant::now();

    let phase_start = std::time::Instant::now();
    let mut random = NollaPrng::new(seed);
    random.set_random_seed(x, y);
    profile.setup += phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let mut wand = get_wand_stats(cost, level, force_unshuffle, &mut random);
    profile.stats += phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    consume_sprite_rng_profiled(&mut random, &wand, profile);
    profile.sprite += phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    wand.spells.clear();
    add_random_cards(&mut wand, seed, x, y, level, &mut random, save_flags);
    profile.cards += phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let capacity = wand.capacity.floor().max(0.0) as usize;
    wand.spells
        .resize(capacity.max(wand.spells.len()), Spell::None);
    let wand = wand.into_public();
    profile.finalize += phase_start.elapsed();

    profile.wands += 1;
    profile.total += total_start.elapsed();
    wand
}
