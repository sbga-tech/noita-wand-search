use crate::data::{
    ActionType, Spell, SpellProb, SPELL_PROBS_COUNTS, SPELL_PROBS_TYPES, WAND_SPRITES,
};
use crate::rng::NollaPrng;
use crate::types::{Wand, WandSpells};

type ProbEntry = (f32, ProbDistribution);

#[derive(Clone, Copy)]
enum ProbDistribution {
    Int {
        min: i32,
        max: i32,
        mean: i32,
        sharpness: f32,
    },
    Float {
        min: f32,
        max: f32,
        mean: f32,
        sharpness: f32,
    },
}

#[derive(Clone, Copy)]
struct ProbTable {
    total_weight: f32,
    entries: &'static [ProbEntry],
}

#[derive(Clone, Copy)]
enum InternalStat {
    Reload,
    CastDelay,
    Spread,
    Speed,
    Capacity,
    Multicast,
    Shuffle,
}

struct InternalWandInst {
    cost: f32,
    force_unshuffle: bool,
    is_rare: bool,
    sprite: usize,
    capacity: f32,
    multicast: i32,
    mana: i32,
    regen: i32,
    delay: i32,
    reload: i32,
    speed: f32,
    spread: i32,
    shuffle: bool,
    always_cast: Spell,
    spells: WandSpells,
}

impl InternalWandInst {
    fn new() -> Self {
        Self {
            cost: 0.0,
            force_unshuffle: false,
            is_rare: false,
            sprite: 0,
            capacity: 0.0,
            multicast: 0,
            mana: 0,
            regen: 0,
            delay: 0,
            reload: 0,
            speed: 0.0,
            spread: 0,
            shuffle: true,
            always_cast: Spell::None,
            spells: WandSpells::new(),
        }
    }

    fn into_public(self) -> Wand {
        Wand {
            capacity: self.capacity.floor() as i32,
            multicast: self.multicast,
            mana: self.mana,
            regen: self.regen,
            delay: self.delay as f64 / 60.0,
            reload: self.reload as f64 / 60.0,
            speed: self.speed,
            spread: self.spread,
            shuffle: self.shuffle,
            always_cast: self.always_cast,
            sprite: self.sprite,
            spells: self.spells,
        }
    }
}

macro_rules! prob_table {
    (int [$(
        (
            $weight:expr,
            {
                min: $min:expr,
                max: $max:expr,
                mean: $mean:expr,
                sharpness: $sharpness:expr $(,)?
            }
        )
    ),+ $(,)?]) => {
        ProbTable {
            total_weight: 0.0 $(+ $weight as f32)+,
            entries: &[
                $(($weight as f32, ProbDistribution::Int {
                    min: $min,
                    max: $max,
                    mean: $mean,
                    sharpness: $sharpness,
                })),+
            ],
        }
    };
    (float [$(
        (
            $weight:expr,
            {
                min: $min:expr,
                max: $max:expr,
                mean: $mean:expr,
                sharpness: $sharpness:expr $(,)?
            }
        )
    ),+ $(,)?]) => {
        ProbTable {
            total_weight: 0.0 $(+ $weight as f32)+,
            entries: &[
                $(($weight as f32, ProbDistribution::Float {
                    min: $min,
                    max: $max,
                    mean: $mean,
                    sharpness: $sharpness,
                })),+
            ],
        }
    };
}

const RELOAD_PROB: ProbTable = prob_table!(int [
    (1.0, { min: 5, max: 60, mean: 30, sharpness: 2.0 }),
    (0.5, { min: 1, max: 100, mean: 40, sharpness: 2.0 }),
    (0.02, { min: 1, max: 100, mean: 40, sharpness: 0.0 }),
    (0.35, { min: 1, max: 240, mean: 40, sharpness: 0.0 }),
]);

const CAST_DELAY_PROB: ProbTable = prob_table!(int [
    (1.0, { min: 1, max: 30, mean: 5, sharpness: 2.0 }),
    (0.1, { min: 1, max: 50, mean: 15, sharpness: 3.0 }),
    (0.1, { min: -15, max: 15, mean: 0, sharpness: 3.0 }),
    (0.45, { min: 0, max: 35, mean: 12, sharpness: 0.0 }),
]);

const SPREAD_PROB: ProbTable = prob_table!(int [
    (1.0, { min: -5, max: 10, mean: 0, sharpness: 3.0 }),
    (0.1, { min: -35, max: 35, mean: 0, sharpness: 0.0 }),
]);

const SPEED_PROB: ProbTable = prob_table!(float [
    (1.0, { min: 0.8, max: 1.2, mean: 1.0, sharpness: 6.0 }),
    (0.05, { min: 1.0, max: 2.0, mean: 1.1, sharpness: 3.0 }),
    (0.05, { min: 0.5, max: 1.0, mean: 0.9, sharpness: 3.0 }),
    (1.0, { min: 0.8, max: 1.2, mean: 1.0, sharpness: 0.0 }),
    (0.001, { min: 1.0, max: 10.0, mean: 5.0, sharpness: 2.0 }),
]);

const CAPACITY_PROB: ProbTable = prob_table!(int [
    (1.0, { min: 3, max: 10, mean: 6, sharpness: 2.0 }),
    (0.1, { min: 2, max: 7, mean: 4, sharpness: 4.0 }),
    (0.05, { min: 1, max: 5, mean: 3, sharpness: 4.0 }),
    (0.15, { min: 5, max: 11, mean: 8, sharpness: 2.0 }),
    (0.12, { min: 2, max: 20, mean: 8, sharpness: 4.0 }),
    (0.15, { min: 3, max: 12, mean: 6, sharpness: 6.0 }),
    (1.0, { min: 1, max: 20, mean: 6, sharpness: 0.0 }),
]);

const MULTICAST_PROB: ProbTable = prob_table!(int [
    (1.0, { min: 1, max: 3, mean: 1, sharpness: 3.0 }),
    (0.2, { min: 2, max: 4, mean: 2, sharpness: 8.0 }),
    (0.05, { min: 1, max: 5, mean: 2, sharpness: 2.0 }),
    (1.0, { min: 1, max: 5, mean: 2, sharpness: 0.0 }),
]);

#[derive(Clone, Debug)]
pub struct SaveFlags {
    flags: Vec<String>,
}

impl SaveFlags {
    pub fn new(flags: Vec<String>) -> Self {
        Self { flags }
    }

    pub fn is_spell_unlocked(&self, spell: Spell) -> bool {
        match spell.unlock_flag() {
            None => true,
            Some(flag) => self.flags.iter().any(|candidate| candidate == flag),
        }
    }
}

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

impl ProbTable {
    fn roll_distribution(&self, random: &mut NollaPrng) -> ProbDistribution {
        debug_assert!(
            !self.entries.is_empty(),
            "probability table must contain at least one entry"
        );
        let mut rnd = random.next_f32() * self.total_weight;
        for (weight, distribution) in self.entries {
            if rnd < *weight {
                return *distribution;
            }
            rnd -= *weight;
        }
        self.entries
            .last()
            .map(|(_, distribution)| *distribution)
            .expect("probability table must contain at least one entry")
    }

    fn roll_i32(&self, random: &mut NollaPrng) -> i32 {
        match self.roll_distribution(random) {
            ProbDistribution::Int {
                min,
                max,
                mean,
                sharpness,
            } => random.random_distribution_i32(min, max, mean, sharpness),
            ProbDistribution::Float { .. } => {
                unreachable!("roll_i32 called on a float probability table")
            }
        }
    }

    fn roll_f32(&self, random: &mut NollaPrng) -> f32 {
        match self.roll_distribution(random) {
            ProbDistribution::Int { .. } => {
                unreachable!("roll_f32 called on an integer probability table")
            }
            ProbDistribution::Float {
                min,
                max,
                mean,
                sharpness,
            } => random.random_distribution_f32(min, max, mean, sharpness),
        }
    }
}

fn shuffle_table(table: &mut [InternalStat], random: &mut NollaPrng) {
    for i in (1..table.len()).rev() {
        let j = random.random_i32_inclusive(0, i as i32) as usize;
        table.swap(i, j);
    }
}

fn apply_reload(gun: &mut InternalWandInst, rolled: i32) {
    let min = (60.0 - gun.cost * 5.0).clamp(1.0, 240.0);
    gun.reload = (rolled as f32).clamp(min, 1024.0) as i32;
    gun.cost -= (60 - gun.reload) as f32 / 5.0;
}
fn apply_delay(gun: &mut InternalWandInst, rolled: i32) {
    let min = (16.0 - gun.cost).clamp(-50.0, 50.0);
    gun.delay = (rolled as f32).clamp(min, 50.0) as i32;
    gun.cost -= (16 - gun.delay) as f32;
}
fn apply_spread(gun: &mut InternalWandInst, rolled: i32) {
    let min = (gun.cost / -1.5).clamp(-35.0, 35.0);
    gun.spread = (rolled as f32).clamp(min, 35.0) as i32;
    gun.cost -= (16 - gun.spread) as f32;
}
fn apply_speed(gun: &mut InternalWandInst, rolled: f32) {
    gun.speed = rolled;
}
fn apply_capacity(gun: &mut InternalWandInst, rolled: i32) {
    let mut max = (gun.cost / 5.0 + 6.0).clamp(1.0, 20.0);
    if gun.force_unshuffle {
        max = (gun.cost - 15.0) / 5.0;
        if max > 6.0 {
            max = 6.0 + (gun.cost - 45.0) / 10.0;
        }
    }
    max = max.clamp(1.0, 20.0);
    gun.capacity = (rolled as f32).clamp(1.0, max);
    gun.cost -= (gun.capacity - 6.0) * 5.0;
}
fn apply_multicast(gun: &mut InternalWandInst, rolled: i32) {
    let action_costs = [
        0.0,
        5.0 + gun.capacity * 2.0,
        15.0 + gun.capacity * 3.5,
        35.0 + gun.capacity * 5.0,
        45.0 + gun.capacity * gun.capacity,
    ];
    let mut max = 1.0;
    for (i, cost) in action_costs.iter().enumerate() {
        if *cost <= gun.cost {
            max = i as f32 + 1.0;
        }
    }
    max = max.clamp(1.0, gun.capacity);
    gun.multicast = (rolled as f32).clamp(1.0, max).floor() as i32;
    let idx = gun.multicast.clamp(1, 5) as usize - 1;
    gun.cost -= action_costs[idx];
}
fn apply_shuffle(gun: &mut InternalWandInst, random: &mut NollaPrng) {
    let mut rnd = random.random_i32_inclusive(0, 1);
    if gun.force_unshuffle {
        rnd = 1;
    }
    if rnd == 1 && gun.cost >= 15.0 + gun.capacity * 5.0 && gun.capacity <= 9.0 {
        gun.shuffle = false;
        gun.cost -= 15.0 + gun.capacity * 5.0;
    }
}
fn apply_random_variable(gun: &mut InternalWandInst, s: InternalStat, random: &mut NollaPrng) {
    match s {
        InternalStat::Reload => apply_reload(gun, RELOAD_PROB.roll_i32(random)),
        InternalStat::CastDelay => apply_delay(gun, CAST_DELAY_PROB.roll_i32(random)),
        InternalStat::Spread => apply_spread(gun, SPREAD_PROB.roll_i32(random)),
        InternalStat::Speed => apply_speed(gun, SPEED_PROB.roll_f32(random)),
        InternalStat::Capacity => apply_capacity(gun, CAPACITY_PROB.roll_i32(random)),
        InternalStat::Multicast => apply_multicast(gun, MULTICAST_PROB.roll_i32(random)),
        InternalStat::Shuffle => apply_shuffle(gun, random),
    }
}

fn get_wand_stats(
    cost: i32,
    level: i32,
    force_unshuffle: bool,
    random: &mut NollaPrng,
) -> InternalWandInst {
    let mut gun = InternalWandInst::new();
    let mut cost = cost;
    if level == 1 && random.random_i32_inclusive(0, 100) < 50 {
        cost += 5;
    }
    cost += random.random_i32_inclusive(-3, 3);
    gun.cost = cost as f32;
    gun.regen = 50 * level + random.random_i32_inclusive(-5, 5 * level);
    gun.mana = 50 + 150 * level + random.random_i32_inclusive(-5, 5) * 10;
    let mut p = random.random_i32_inclusive(0, 100);
    if p < 20 {
        gun.regen = (50 * level + random.random_i32_inclusive(-5, 5 * level)) / 5;
        gun.mana = (50 + 150 * level + random.random_i32_inclusive(5, 5) * 10) * 3;
    }
    p = random.random_i32_inclusive(0, 100);
    if p < 15 {
        gun.regen = (50 * level + random.random_i32_inclusive(-5, 5 * level)) * 5;
        gun.mana = (50 + 150 * level + random.random_i32_inclusive(-5, 5) * 10) / 3;
    }
    if gun.mana < 50 {
        gun.mana = 50;
    }
    if gun.regen < 10 {
        gun.regen = 10;
    }
    p = random.random_i32_inclusive(0, 100);
    if p < 15 + level * 6 {
        gun.force_unshuffle = true;
    }
    p = random.random_i32_inclusive(0, 100);
    if p < 5 {
        gun.is_rare = true;
        gun.cost += 65.0;
    }
    let mut variables_01 = [
        InternalStat::Reload,
        InternalStat::CastDelay,
        InternalStat::Spread,
        InternalStat::Speed,
    ];
    shuffle_table(&mut variables_01, random);
    let multicast_first = !gun.force_unshuffle && random.random_i32_inclusive(0, 1) == 0;
    for s in variables_01 {
        apply_random_variable(&mut gun, s, random);
    }
    apply_random_variable(&mut gun, InternalStat::Capacity, random);
    if multicast_first {
        apply_random_variable(&mut gun, InternalStat::Multicast, random);
        apply_random_variable(&mut gun, InternalStat::Shuffle, random);
    } else {
        apply_random_variable(&mut gun, InternalStat::Shuffle, random);
        apply_random_variable(&mut gun, InternalStat::Multicast, random);
    }
    if gun.cost > 5.0 && random.random_i32_inclusive(0, 1000) < 995 {
        if gun.shuffle {
            gun.capacity += gun.cost / 5.0;
        } else {
            gun.capacity += gun.cost / 10.0;
        }
        gun.cost = 0.0;
    }
    if force_unshuffle {
        gun.shuffle = false;
    }
    if random.random_i32_inclusive(0, 10000) <= 9999 {
        gun.capacity = gun.capacity.clamp(2.0, 26.0);
    }
    gun.capacity = gun.capacity.max(2.0);
    if gun.reload >= 60 {
        let mut rnd = 0;
        while rnd < 70 {
            gun.multicast += 1;
            rnd = random.random_i32_inclusive(0, 100);
        }
        if random.random_i32_inclusive(0, 100) < 50 {
            let mut new_multicast = gun.capacity as i32;
            for _ in 1..=6 {
                let temp = random.random_i32_inclusive(gun.multicast, gun.capacity as i32);
                if temp < new_multicast {
                    new_multicast = temp;
                }
            }
            gun.multicast = new_multicast;
        }
    }
    gun.multicast = gun.multicast.clamp(1, gun.capacity as i32);
    gun
}

fn get_best_sprite(random: &mut NollaPrng, w: &mut InternalWandInst) {
    let fire_rate_wait = (((w.delay + 5) as f32 / 7.0) - 1.0).clamp(0.0, 4.0);
    let actions_per_round = (w.multicast - 1).clamp(0, 2) as f32;
    let deck_capacity = ((w.capacity - 3.0) / 3.0).clamp(0.0, 7.0);
    let spread_degrees = (((w.spread + 5) as f32 / 5.0) - 1.0).clamp(0.0, 2.0);
    let reload_time = (((w.reload + 5) as f32 / 25.0) - 1.0).clamp(0.0, 2.0);
    let mut best_score = 1000.0_f32;
    for (i, sprite) in WAND_SPRITES.iter().enumerate() {
        let mut score = 0.0;
        score += (fire_rate_wait - sprite.fire_rate_wait as f32).abs() * 2.0;
        score += (actions_per_round - sprite.actions_per_round as f32).abs() * 20.0;
        score += ((w.shuffle as i32 - sprite.shuffle_deck_when_empty as i32).abs() as f32) * 30.0;
        score += (deck_capacity - sprite.deck_capacity as f32).abs() * 5.0;
        score += (spread_degrees - sprite.spread_degrees as f32).abs();
        score += (reload_time - sprite.reload_time as f32).abs();
        if score <= best_score {
            best_score = score;
            w.sprite = i;
            if score == 0.0 && random.random_i32_inclusive(0, 100) < 33 {
                break;
            }
        }
    }
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

fn add_random_cards(
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
    let mut card_count = random
        .random_f32(0.51 * capacity, capacity)
        .floor()
        .clamp(1.0, (capacity - 1.0).floor()) as i32;
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
    get_best_sprite(&mut random, &mut wand);
    wand.spells.clear();
    add_random_cards(&mut wand, seed, x, y, level, &mut random, save_flags);
    let capacity = wand.capacity.floor().max(0.0) as usize;
    wand.spells
        .resize(capacity.max(wand.spells.len()), Spell::None);
    wand.into_public()
}
