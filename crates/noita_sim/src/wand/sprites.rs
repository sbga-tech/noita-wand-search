#[cfg(feature = "profiling")]
use super::profiling::WandGenerationPhaseProfile;
use super::stats::InternalWandInst;
use crate::data::WAND_SPRITES;
use crate::rng::NollaPrng;

#[derive(Clone, Copy, Debug)]
struct SpriteFeatures {
    fire_rate_wait: f32,
    actions_per_round: f32,
    shuffle: bool,
    deck_capacity: f32,
    spread_degrees: f32,
    reload_time: f32,
}

const FIRE_RATE_WAIT_BUCKETS: usize = 5;
const ACTIONS_PER_ROUND_BUCKETS: usize = 3;
const SHUFFLE_BUCKETS: usize = 2;
const DECK_CAPACITY_BUCKETS: usize = 8;
const SPREAD_DEGREES_BUCKETS: usize = 3;
const RELOAD_TIME_BUCKETS: usize = 3;
const EXACT_SPRITE_KEY_COUNT: usize = FIRE_RATE_WAIT_BUCKETS
    * ACTIONS_PER_ROUND_BUCKETS
    * SHUFFLE_BUCKETS
    * DECK_CAPACITY_BUCKETS
    * SPREAD_DEGREES_BUCKETS
    * RELOAD_TIME_BUCKETS;

struct ExactSpriteLUT {
    offsets: [u16; EXACT_SPRITE_KEY_COUNT + 1],
    indices: Vec<u16>,
}

static EXACT_SPRITE_LUT: std::sync::LazyLock<ExactSpriteLUT> =
    std::sync::LazyLock::new(build_exact_sprite_lut);

fn is_exact_feature_bucket(value: f32, max: u8) -> bool {
    value.fract() == 0.0 && value >= 0.0 && value <= max as f32
}

fn is_possible_exact_sprite(features: SpriteFeatures) -> bool {
    is_exact_feature_bucket(features.fire_rate_wait, 4)
        && is_exact_feature_bucket(features.actions_per_round, 2)
        && is_exact_feature_bucket(features.deck_capacity, 7)
        && is_exact_feature_bucket(features.spread_degrees, 2)
        && is_exact_feature_bucket(features.reload_time, 2)
}

fn sprite_key_index(features: SpriteFeatures) -> usize {
    debug_assert!(is_possible_exact_sprite(features));
    let shuffle = usize::from(features.shuffle);
    (((((features.fire_rate_wait as usize * ACTIONS_PER_ROUND_BUCKETS
        + features.actions_per_round as usize)
        * SHUFFLE_BUCKETS
        + shuffle)
        * DECK_CAPACITY_BUCKETS
        + features.deck_capacity as usize)
        * SPREAD_DEGREES_BUCKETS
        + features.spread_degrees as usize)
        * RELOAD_TIME_BUCKETS)
        + features.reload_time as usize
}

fn sprite_data_features(sprite: &crate::data::WandSprite) -> SpriteFeatures {
    SpriteFeatures {
        fire_rate_wait: sprite.fire_rate_wait as f32,
        actions_per_round: sprite.actions_per_round as f32,
        shuffle: sprite.shuffle_deck_when_empty,
        deck_capacity: sprite.deck_capacity as f32,
        spread_degrees: sprite.spread_degrees as f32,
        reload_time: sprite.reload_time as f32,
    }
}

fn build_exact_sprite_lut() -> ExactSpriteLUT {
    let mut counts = [0_u16; EXACT_SPRITE_KEY_COUNT];
    for sprite in WAND_SPRITES {
        counts[sprite_key_index(sprite_data_features(sprite))] += 1;
    }

    let mut offsets = [0_u16; EXACT_SPRITE_KEY_COUNT + 1];
    for i in 0..EXACT_SPRITE_KEY_COUNT {
        offsets[i + 1] = offsets[i] + counts[i];
    }

    let mut write_offsets = offsets;
    let mut indices = vec![0_u16; WAND_SPRITES.len()];
    for (sprite_index, sprite) in WAND_SPRITES.iter().enumerate() {
        let key_index = sprite_key_index(sprite_data_features(sprite));
        let write_index = write_offsets[key_index] as usize;
        indices[write_index] = sprite_index as u16;
        write_offsets[key_index] += 1;
    }

    ExactSpriteLUT { offsets, indices }
}

fn matching_sprite_indices(features: SpriteFeatures) -> &'static [u16] {
    debug_assert!(is_possible_exact_sprite(features));
    let lookup = &*EXACT_SPRITE_LUT;
    let key_index = sprite_key_index(features);
    let start = lookup.offsets[key_index] as usize;
    let end = lookup.offsets[key_index + 1] as usize;
    &lookup.indices[start..end]
}

fn select_exact_sprite(random: &mut NollaPrng, features: SpriteFeatures) -> Option<usize> {
    let mut selected = None;
    for index in matching_sprite_indices(features) {
        selected = Some(*index as usize);
        if random.random_i32_inclusive(0, 100) < 33 {
            break;
        }
    }
    selected
}

fn sprite_features(w: &InternalWandInst) -> SpriteFeatures {
    SpriteFeatures {
        fire_rate_wait: (((w.delay + 5) as f32 / 7.0) - 1.0).clamp(0.0, 4.0),
        actions_per_round: (w.multicast - 1).clamp(0, 2) as f32,
        shuffle: w.shuffle,
        deck_capacity: ((w.capacity - 3.0) / 3.0).clamp(0.0, 7.0),
        spread_degrees: (((w.spread + 5) as f32 / 5.0) - 1.0).clamp(0.0, 2.0),
        reload_time: (((w.reload + 5) as f32 / 25.0) - 1.0).clamp(0.0, 2.0),
    }
}

fn sprite_score(features: SpriteFeatures, sprite: &crate::data::WandSprite) -> f32 {
    let mut score = 0.0;
    score += (features.fire_rate_wait - sprite.fire_rate_wait as f32).abs() * 2.0;
    score += (features.actions_per_round - sprite.actions_per_round as f32).abs() * 20.0;
    score +=
        ((features.shuffle as i32 - sprite.shuffle_deck_when_empty as i32).abs() as f32) * 30.0;
    score += (features.deck_capacity - sprite.deck_capacity as f32).abs() * 5.0;
    score += (features.spread_degrees - sprite.spread_degrees as f32).abs();
    score += (features.reload_time - sprite.reload_time as f32).abs();
    score
}

pub(in crate::wand) fn select_best_sprite(random: &mut NollaPrng, w: &InternalWandInst) -> usize {
    let features = sprite_features(w);
    if is_possible_exact_sprite(features) {
        if let Some(sprite) = select_exact_sprite(random, features) {
            return sprite;
        }
    }

    let mut best_score = 1000.0_f32;
    let mut best_sprite = 0;
    for (i, sprite) in WAND_SPRITES.iter().enumerate() {
        let score = sprite_score(features, sprite);
        if score <= best_score {
            best_score = score;
            best_sprite = i;
            if score == 0.0 && random.random_i32_inclusive(0, 100) < 33 {
                break;
            }
        }
    }
    best_sprite
}

pub(in crate::wand) fn consume_sprite_rng(random: &mut NollaPrng, w: &InternalWandInst) {
    let features = sprite_features(w);
    if !is_possible_exact_sprite(features) {
        return;
    }
    for _ in matching_sprite_indices(features) {
        if random.random_i32_inclusive(0, 100) < 33 {
            break;
        }
    }
}

#[cfg(feature = "profiling")]
pub(in crate::wand) fn consume_sprite_rng_profiled(
    random: &mut NollaPrng,
    w: &InternalWandInst,
    profile: &mut WandGenerationPhaseProfile,
) {
    let features = sprite_features(w);
    if !is_possible_exact_sprite(features) {
        return;
    }
    let mut saw_zero_match = false;
    for _ in matching_sprite_indices(features) {
        saw_zero_match = true;
        profile.sprite_zero_matches += 1;
        profile.sprite_zero_rng_draws += 1;
        if random.random_i32_inclusive(0, 100) < 33 {
            break;
        }
    }
    if saw_zero_match {
        profile.sprite_zero_match_wands += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::stats::get_wand_stats;
    use super::*;

    #[test]
    fn exact_sprite_fast_path_matches_linear_selector() {
        for seed in 0..512 {
            let mut random = NollaPrng::new(seed);
            random.set_random_seed(seed as f64 * 0.5, seed as f64 * -0.25);
            let wand = get_wand_stats(100, 5, false, false, &mut random);
            let mut linear_random = random.clone();
            let mut lookup_random = random;

            let features = sprite_features(&wand);
            let mut best_score = 1000.0_f32;
            let mut linear_sprite = 0;
            for (i, sprite) in WAND_SPRITES.iter().enumerate() {
                let score = sprite_score(features, sprite);
                if score <= best_score {
                    best_score = score;
                    linear_sprite = i;
                    if score == 0.0 && linear_random.random_i32_inclusive(0, 100) < 33 {
                        break;
                    }
                }
            }

            assert_eq!(
                linear_sprite,
                select_best_sprite(&mut lookup_random, &wand),
                "sprite selector mismatch for seed {seed}"
            );
        }
    }

    #[test]
    fn possible_exact_sprite_rejects_fractional_and_out_of_range_values() {
        let exact_min = SpriteFeatures {
            fire_rate_wait: 0.0,
            actions_per_round: 0.0,
            shuffle: false,
            deck_capacity: 0.0,
            spread_degrees: 0.0,
            reload_time: 0.0,
        };
        assert!(is_possible_exact_sprite(exact_min));
        assert!(is_possible_exact_sprite(SpriteFeatures {
            fire_rate_wait: 4.0,
            actions_per_round: 2.0,
            shuffle: true,
            deck_capacity: 7.0,
            spread_degrees: 2.0,
            reload_time: 2.0,
        }));
        assert!(!is_possible_exact_sprite(SpriteFeatures {
            fire_rate_wait: 1.5,
            ..exact_min
        }));
        assert!(!is_possible_exact_sprite(SpriteFeatures {
            fire_rate_wait: -1.0,
            ..exact_min
        }));
        assert!(!is_possible_exact_sprite(SpriteFeatures {
            fire_rate_wait: 5.0,
            ..exact_min
        }));
    }

    #[test]
    fn sprite_exact_lookup_matches_linear_scan_order() {
        for fire_rate_wait in 0..FIRE_RATE_WAIT_BUCKETS as u8 {
            for actions_per_round in 0..ACTIONS_PER_ROUND_BUCKETS as u8 {
                for shuffle in [false, true] {
                    for deck_capacity in 0..DECK_CAPACITY_BUCKETS as u8 {
                        for spread_degrees in 0..SPREAD_DEGREES_BUCKETS as u8 {
                            for reload_time in 0..RELOAD_TIME_BUCKETS as u8 {
                                let features = SpriteFeatures {
                                    fire_rate_wait: fire_rate_wait as f32,
                                    actions_per_round: actions_per_round as f32,
                                    shuffle,
                                    deck_capacity: deck_capacity as f32,
                                    spread_degrees: spread_degrees as f32,
                                    reload_time: reload_time as f32,
                                };
                                let lookup = matching_sprite_indices(features)
                                    .iter()
                                    .map(|index| *index as usize)
                                    .collect::<Vec<_>>();
                                let scan = WAND_SPRITES
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(index, sprite)| {
                                        (sprite_score(features, sprite) == 0.0).then_some(index)
                                    })
                                    .collect::<Vec<_>>();
                                assert_eq!(
                                    lookup, scan,
                                    "lookup mismatch for features {features:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
