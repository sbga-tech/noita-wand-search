use super::stats::InternalWandInst;
use crate::data::{ActionType, Spell, SpellProb, SPELL_PROBS_COUNTS, SPELL_PROBS_TYPES};
use crate::rng::NollaPrng;
use crate::types::SaveFlags;

fn get_random_from_probs(probs: &[SpellProb], random: &mut NollaPrng, add_epsilon: bool) -> Spell {
    if probs.is_empty() {
        return Spell::None;
    }
    let sum = probs.last().unwrap().p;
    let cutoff = random.next_f64() * sum + if add_epsilon { 0.0001 } else { 0.0 };
    let mut low = 0usize;
    let mut high = probs.len();
    while low < high {
        let mid = low + (high - low) / 2;
        if probs[mid].p < cutoff {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    probs[low.min(probs.len() - 1)].spell
}

fn get_random_from_probs_unlocked(
    probs: &[SpellProb],
    random: &mut NollaPrng,
    add_epsilon: bool,
    save_flags: &SaveFlags,
) -> Spell {
    let mut total = 0.0;
    let mut previous = 0.0;
    for prob in probs {
        let weight = prob.p - previous;
        previous = prob.p;
        if save_flags.is_spell_unlocked(prob.spell) {
            total += weight;
        }
    }
    if total <= 0.0 {
        return Spell::None;
    }
    let mut cutoff = random.next_f64() * total + if add_epsilon { 0.0001 } else { 0.0 };
    previous = 0.0;
    for prob in probs {
        let weight = prob.p - previous;
        previous = prob.p;
        if save_flags.is_spell_unlocked(prob.spell) {
            if cutoff <= weight {
                return prob.spell;
            }
            cutoff -= weight;
        }
    }
    Spell::None
}

fn choose_random_spell_from_probs(
    seed: u32,
    x: f64,
    y: f64,
    offset: i32,
    probs: &[SpellProb],
    save_flags: Option<&SaveFlags>,
) -> Spell {
    let mut random = NollaPrng::new(seed.wrapping_add(offset as u32));
    random.set_random_seed(x, y);
    match save_flags {
        Some(save_flags) => get_random_from_probs_unlocked(probs, &mut random, true, save_flags),
        None => get_random_from_probs(probs, &mut random, true),
    }
}

fn get_random_action_with_type(
    seed: u32,
    x: f64,
    y: f64,
    level: i32,
    action_type: ActionType,
    offset: i32,
    save_flags: Option<&SaveFlags>,
) -> Spell {
    let level = level.clamp(0, 10) as usize;
    let action_type = usize::from(u8::from(action_type));
    if SPELL_PROBS_COUNTS[level][action_type] == 0 {
        return Spell::None;
    }
    choose_random_spell_from_probs(
        seed,
        x,
        y,
        offset,
        SPELL_PROBS_TYPES[level][action_type],
        save_flags,
    )
}

fn consume_discarded_card_rolls(random: &mut NollaPrng, is_rare: bool) {
    if random.random_i32_inclusive(0, 100) < 7 {
        let _ = random.random_i32_inclusive(20, 50);
    }

    let discarded_card_count = random.random_i32_inclusive(1, 3);
    let _ = random.random_i32_inclusive(0, 100) < 50 && discarded_card_count < 3;
    if random.random_i32_inclusive(0, 100) < 10 || is_rare {
        let _ = random.random_i32_inclusive(1, 2);
    }
}

pub(in crate::wand) fn add_random_cards(
    gun: &mut InternalWandInst,
    seed: u32,
    x: f64,
    y: f64,
    level_raw: i32,
    random: &mut NollaPrng,
    save_flags: Option<&SaveFlags>,
) {
    let is_rare = gun.is_rare;
    consume_discarded_card_rolls(random, is_rare);
    let orig_level = level_raw;
    let level = level_raw - 1;
    let capacity = gun.capacity;
    let multicast = gun.multicast;
    let mut bullet_card =
        get_random_action_with_type(seed, x, y, level, ActionType::Projectile, 0, save_flags);
    let mut card;
    let mut random_bullets = 0;
    let mut good_card_count = 0;
    let good_cards = random.random_i32_inclusive(5, 45);
    let min_card_count = (0.51 * capacity).ceil() as i32;
    let max_card_count = capacity.floor() as i32;
    let mut card_count = random
        .random_i32_inclusive(min_card_count, max_card_count)
        .clamp(1, (capacity - 1.0).floor() as i32);
    if random.random_i32_inclusive(0, 100) < orig_level * 10 - 5 {
        random_bullets = 1;
    }
    if random.random_i32_inclusive(0, 100) < 4 || is_rare {
        let p = random.random_i32_inclusive(0, 100);
        if p < 77 {
            card = get_random_action_with_type(
                seed,
                x,
                y,
                level + 1,
                ActionType::Modifier,
                666,
                save_flags,
            );
        } else if p < 85 {
            card = get_random_action_with_type(
                seed,
                x,
                y,
                level + 1,
                ActionType::Modifier,
                666,
                save_flags,
            );
            good_card_count += 1;
        } else if p < 93 {
            card = get_random_action_with_type(
                seed,
                x,
                y,
                level + 1,
                ActionType::StaticProjectile,
                666,
                save_flags,
            );
        } else {
            card = get_random_action_with_type(
                seed,
                x,
                y,
                level + 1,
                ActionType::Projectile,
                666,
                save_flags,
            );
        }
        gun.always_cast = card;
    } else {
        gun.always_cast = Spell::None;
    }
    if random.random_i32_inclusive(0, 100) < 50 {
        let mut extra_level = level;
        while random.random_i32_inclusive(1, 10) == 10 {
            extra_level += 1;
            bullet_card = get_random_action_with_type(
                seed,
                x,
                y,
                extra_level,
                ActionType::Projectile,
                0,
                save_flags,
            );
        }
        if card_count < 3 {
            if card_count > 1 && random.random_i32_inclusive(0, 100) < 20 {
                card = get_random_action_with_type(
                    seed,
                    x,
                    y,
                    level,
                    ActionType::Modifier,
                    2,
                    save_flags,
                );
                gun.spells.push(card);
                card_count -= 1;
            }
            for _ in 0..card_count {
                gun.spells.push(bullet_card);
            }
        } else {
            if random.random_i32_inclusive(0, 100) < 40 {
                card = get_random_action_with_type(
                    seed,
                    x,
                    y,
                    level,
                    ActionType::DrawMany,
                    1,
                    save_flags,
                );
                gun.spells.push(card);
                card_count -= 1;
            }
            if card_count > 3 && random.random_i32_inclusive(0, 100) < 40 {
                card = get_random_action_with_type(
                    seed,
                    x,
                    y,
                    level,
                    ActionType::DrawMany,
                    1,
                    save_flags,
                );
                gun.spells.push(card);
                card_count -= 1;
            }
            if random.random_i32_inclusive(0, 100) < 80 {
                card = get_random_action_with_type(
                    seed,
                    x,
                    y,
                    level,
                    ActionType::Modifier,
                    2,
                    save_flags,
                );
                gun.spells.push(card);
                card_count -= 1;
            }
            for _ in 0..card_count {
                gun.spells.push(bullet_card);
            }
        }
    } else {
        for i in 0..card_count {
            if random.random_i32_inclusive(0, 100) < good_cards && card_count > 2 {
                if good_card_count == 0 && multicast == 1 {
                    card = get_random_action_with_type(
                        seed,
                        x,
                        y,
                        level,
                        ActionType::DrawMany,
                        i + 1,
                        save_flags,
                    );
                    good_card_count += 1;
                } else if random.random_i32_inclusive(0, 100) < 83 {
                    card = get_random_action_with_type(
                        seed,
                        x,
                        y,
                        level,
                        ActionType::Modifier,
                        i + 1,
                        save_flags,
                    );
                } else {
                    card = get_random_action_with_type(
                        seed,
                        x,
                        y,
                        level,
                        ActionType::DrawMany,
                        i + 1,
                        save_flags,
                    );
                }
                gun.spells.push(card);
            } else {
                gun.spells.push(bullet_card);
                if random_bullets == 1 {
                    bullet_card = get_random_action_with_type(
                        seed,
                        x,
                        y,
                        level,
                        ActionType::Projectile,
                        i + 1,
                        save_flags,
                    );
                }
            }
        }
    }
}
