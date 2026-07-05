use crate::data::Spell;
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

pub(in crate::wand) struct InternalWandInst {
    pub(in crate::wand) cost: f32,
    pub(in crate::wand) force_unshuffle: bool,
    pub(in crate::wand) is_rare: bool,
    pub(in crate::wand) capacity: f32,
    pub(in crate::wand) multicast: i32,
    pub(in crate::wand) mana: i32,
    pub(in crate::wand) regen: i32,
    pub(in crate::wand) delay: i32,
    pub(in crate::wand) reload: i32,
    pub(in crate::wand) speed: f32,
    pub(in crate::wand) spread: i32,
    pub(in crate::wand) shuffle: bool,
    pub(in crate::wand) always_cast: Spell,
    pub(in crate::wand) spells: WandSpells,
}

impl InternalWandInst {
    fn new() -> Self {
        Self {
            cost: 0.0,
            force_unshuffle: false,
            is_rare: false,
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

    pub(in crate::wand) fn into_public(self) -> Wand {
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

pub(in crate::wand) fn get_wand_stats(
    cost: i32,
    level: i32,
    force_unshuffle: bool,
    no_more_shuffle_wands: bool,
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
        gun.mana = (50 + 150 * level + random.random_i32_inclusive(-5, 5) * 10) * 3;
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
    if force_unshuffle || no_more_shuffle_wands {
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
