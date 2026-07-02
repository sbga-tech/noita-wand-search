use crate::data::Material;
use crate::potions::{create_potion, PotionKind};
use crate::rng::NollaPrng;
use crate::types::Wand;
use crate::wandgen::{get_wand_unlocked, SaveFlags};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WandSpawner {
    pub cost: i32,
    pub level: i32,
    pub forced_nonshuffle: bool,
    pub x_offset: f64,
    pub y_offset: f64,
}

impl WandSpawner {
    pub const fn new(cost: i32, level: i32, forced_nonshuffle: bool) -> Self {
        Self {
            cost,
            level,
            forced_nonshuffle,
            x_offset: 0.0,
            y_offset: 0.0,
        }
    }

    pub const fn with_offset(mut self, x_offset: f64, y_offset: f64) -> Self {
        self.x_offset = x_offset;
        self.y_offset = y_offset;
        self
    }

    pub const fn spell_level(self) -> i32 {
        self.level
    }

    fn spawn_wand(
        &self,
        world_seed: u32,
        coord: &SpawnCoord,
        save_flags: Option<&SaveFlags>,
    ) -> Wand {
        get_wand_unlocked(
            world_seed,
            coord.x as f64 + self.x_offset,
            coord.y as f64 + self.y_offset,
            self.cost,
            self.level,
            self.forced_nonshuffle,
            save_flags,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Wand(Wand),
    Material(Material),
    Placeholder(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnCoord {
    pub x: i32,
    pub y: i32,
}

enum ItemSpawner {
    Wand(WandSpawner),
    Potions(&'static [PotionKind]),
}

enum LootNode {
    Spawner(ItemSpawner),
    ItemPlaceholder(&'static str),
    Table(&'static LootTable),
    Reroll(u32),
}

struct LootEntry {
    threshold: i32,
    node: LootNode,
}

#[derive(Clone, Copy)]
pub struct LootTable {
    min_roll: i32,
    max_roll: i32,
    entries: &'static [LootEntry],
}

impl LootTable {
    fn roll(&self, random: &mut NollaPrng) -> &LootNode {
        let roll = random.random_i32_inclusive(self.min_roll, self.max_roll);
        for entry in self.entries {
            if roll <= entry.threshold {
                return &entry.node;
            }
        }
        unreachable!("loot table roll exceeded its final threshold")
    }
}

macro_rules! loot_table {
    ({
        min_roll: $min_roll:expr,
        max_roll: $max_roll:expr,
        [
            $(
                {
                    threshold: $threshold:expr,
                    node: $node:expr $(,)?
                }
            ),+ $(,)?
        ]
    }) => {{
        const TABLE: LootTable = LootTable {
            min_roll: $min_roll,
            max_roll: $max_roll,
            entries: &[
                $(LootEntry {
                    threshold: $threshold,
                    node: $node,
                }),+
            ],
        };
        const _: () = assert!(loot_table_is_valid(TABLE));
        TABLE
    }};
}

macro_rules! loot_table_weight {
    ($table:expr) => {{
        const TABLE: LootTable = $table;
        const _: () = assert!(wand_spawner_table_is_valid(TABLE));
        wand_spawner_weights_from_table::<{ TABLE.entries.len() }>(TABLE)
    }};
}

const POTION_STANDARD_STANDARD_SECRET: [PotionKind; 3] = [
    PotionKind::StandardOrMagic,
    PotionKind::StandardOrMagic,
    PotionKind::Secret,
];

const POTION_SECRET_SECRET_RANDOM_MATERIAL: [PotionKind; 3] = [
    PotionKind::Secret,
    PotionKind::Secret,
    PotionKind::LiquidOrSand,
];

const GREAT_CHEST_POTION_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 100,
    [
        {
            threshold: 30,
            node: LootNode::Spawner(ItemSpawner::Potions(&POTION_STANDARD_STANDARD_SECRET)),
        },
        {
            threshold: 100,
            node: LootNode::Spawner(ItemSpawner::Potions(
                &POTION_SECRET_SECRET_RANDOM_MATERIAL,
            )),
        },
    ]
});

const GREAT_CHEST_STONE_TABLE: LootTable = loot_table!({
    min_roll: 1,
    max_roll: 30,
    [
        {
            threshold: 29,
            // TODO: implement waterstone item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("waterstone"),
        },
        {
            threshold: 30,
            // TODO: implement poopstone item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("poopstone"),
        },
    ]
});

const GREAT_CHEST_WAND_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 100,
    [
        {
            threshold: 25,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(80, 4, false))),
        },
        {
            threshold: 50,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(80, 4, true))),
        },
        {
            threshold: 75,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(100, 5, false))),
        },
        {
            threshold: 90,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(100, 5, true))),
        },
        {
            threshold: 96,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(120, 6, false))),
        },
        {
            threshold: 98,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(120, 6, true))),
        },
        {
            threshold: 99,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(120, 6, false))),
        },
        {
            threshold: 100,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(180, 11, true))),
        },
    ]
});

const GREAT_CHEST_HEART_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 100,
    [
        {
            threshold: 89,
            // TODO: implement heart item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("heart"),
        },
        {
            threshold: 99,
            // TODO: implement better-heart item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("heart_better"),
        },
        {
            threshold: 100,
            // TODO: implement full-heal heart item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("heart_fullhp"),
        },
    ]
});

const GREAT_CHEST_MAIN_TABLE: LootTable = loot_table!({
    min_roll: 1,
    max_roll: 100,
    [
        {
            threshold: 10,
            node: LootNode::Table(&GREAT_CHEST_POTION_TABLE),
        },
        {
            threshold: 15,
            // TODO: implement gold-rain item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("rain_gold"),
        },
        {
            threshold: 18,
            node: LootNode::Table(&GREAT_CHEST_STONE_TABLE),
        },
        {
            threshold: 39,
            node: LootNode::Table(&GREAT_CHEST_WAND_TABLE),
        },
        {
            threshold: 60,
            node: LootNode::Table(&GREAT_CHEST_HEART_TABLE),
        },
        {
            threshold: 98,
            node: LootNode::Reroll(2),
        },
        {
            threshold: 100,
            node: LootNode::Reroll(3),
        },
    ]
});

pub const GREAT_CHEST_LOOT_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 100_000,
    [
        {
            threshold: 99_999,
            node: LootNode::Table(&GREAT_CHEST_MAIN_TABLE),
        },
        {
            threshold: 100_000,
            // TODO: implement sampo/end-game item semantics if the project starts exposing non-wand chest items.
            node: LootNode::ItemPlaceholder("sampo"),
        },
    ]
});

pub const TAIKASAUVA_LOOT_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 0,
    [
        {
            threshold: 0,
            node: LootNode::Spawner(ItemSpawner::Wand(WandSpawner::new(60, 3, false))),
        },
    ]
});

pub const TINY_DROP_LOOT_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 0,
    [
        {
            threshold: 0,
            node: LootNode::Spawner(ItemSpawner::Wand(
                WandSpawner::new(180, 11, true).with_offset(16.0, 0.0),
            )),
        },
    ]
});

const fn loot_table_is_valid(table: LootTable) -> bool {
    let entries = table.entries;
    if entries.is_empty() {
        return false;
    }
    if entries[0].threshold < table.min_roll {
        return false;
    }
    if entries[entries.len() - 1].threshold != table.max_roll {
        return false;
    }
    let mut i = 1;
    while i < entries.len() {
        if entries[i].threshold <= entries[i - 1].threshold {
            return false;
        }
        i += 1;
    }
    true
}

const fn wand_spawner_from_node(node: &LootNode) -> Option<WandSpawner> {
    match node {
        LootNode::Spawner(ItemSpawner::Wand(spawner)) => Some(*spawner),
        _ => None,
    }
}

const fn wand_spawner_table_is_valid(table: LootTable) -> bool {
    if !loot_table_is_valid(table) {
        return false;
    }
    let entries = table.entries;
    let mut i = 0;
    while i < entries.len() {
        if wand_spawner_from_node(&entries[i].node).is_none() {
            return false;
        }
        i += 1;
    }
    true
}

const fn wand_spawner_weights_from_table<const N: usize>(
    table: LootTable,
) -> [(WandSpawner, f64); N] {
    let entries = table.entries;
    let denominator = (table.max_roll - table.min_roll + 1) as f64;
    let mut out = [(WandSpawner::new(0, 0, false), 0.0); N];
    let mut previous_threshold = table.min_roll - 1;
    let mut i = 0;
    while i < N {
        let count = entries[i].threshold - previous_threshold;
        let spawner = match wand_spawner_from_node(&entries[i].node) {
            Some(spawner) => spawner,
            None => WandSpawner::new(0, 0, false),
        };
        out[i] = (spawner, count as f64 / denominator);
        previous_threshold = entries[i].threshold;
        i += 1;
    }
    out
}

pub const GREAT_CHEST_WAND_SPAWNER_WEIGHTS: [(WandSpawner, f64);
    GREAT_CHEST_WAND_TABLE.entries.len()] = loot_table_weight!(GREAT_CHEST_WAND_TABLE);

pub fn great_chest_wand_spawner_weights() -> &'static [(WandSpawner, f64)] {
    &GREAT_CHEST_WAND_SPAWNER_WEIGHTS
}

pub fn round_rng_pos(num: i32) -> i32 {
    if -1_000_000 < num && num < 1_000_000 {
        num
    } else if -10_000_000 < num && num < 10_000_000 {
        (num as f32 / 10.0).round() as i32 * 10
    } else if -100_000_000 < num && num < 100_000_000 {
        (num as f32 / 100.0).round() as i32 * 100
    } else {
        num
    }
}

pub struct LootSpawner {
    world_seed: u32,
    table: &'static LootTable,
    save_flags: Option<SaveFlags>,
}

pub struct LootIter<'a> {
    world_seed: u32,
    coord: SpawnCoord,
    save_flags: Option<&'a SaveFlags>,
    random: NollaPrng,
    pending_tables: Vec<&'static LootTable>,
    pending_potions: Option<core::slice::Iter<'static, PotionKind>>,
}

impl<'a> LootIter<'a> {
    fn new(
        world_seed: u32,
        table: &'static LootTable,
        save_flags: Option<&'a SaveFlags>,
        coord: SpawnCoord,
    ) -> Self {
        let mut random = NollaPrng::new(world_seed);
        if core::ptr::addr_eq(table, &GREAT_CHEST_LOOT_TABLE) {
            random.set_random_seed_int(round_rng_pos(coord.x), coord.y);
        } else {
            random.set_random_seed(coord.x as f64, coord.y as f64);
        }

        Self {
            world_seed,
            coord,
            save_flags,
            random,
            pending_tables: vec![table],
            pending_potions: None,
        }
    }
}

impl Iterator for LootIter<'_> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(kinds) = &mut self.pending_potions {
                if let Some(kind) = kinds.next() {
                    return Some(Item::Material(create_potion(
                        self.coord.x as f64,
                        self.coord.y as f64,
                        self.world_seed,
                        *kind,
                    )));
                }
                self.pending_potions = None;
            }

            let table = self.pending_tables.pop()?;
            match table.roll(&mut self.random) {
                LootNode::Spawner(ItemSpawner::Wand(spawner)) => {
                    return Some(Item::Wand(spawner.spawn_wand(
                        self.world_seed,
                        &self.coord,
                        self.save_flags,
                    )));
                }
                LootNode::Spawner(ItemSpawner::Potions(kinds)) => {
                    self.pending_potions = Some(kinds.iter());
                }
                LootNode::ItemPlaceholder(name) => return Some(Item::Placeholder(name)),
                LootNode::Table(nested) => self.pending_tables.push(nested),
                LootNode::Reroll(times) => {
                    for _ in 0..*times {
                        self.pending_tables.push(table);
                    }
                }
            }
        }
    }
}

impl LootSpawner {
    pub const fn new(
        world_seed: u32,
        table: &'static LootTable,
        save_flags: Option<SaveFlags>,
    ) -> Self {
        Self {
            world_seed,
            table,
            save_flags,
        }
    }

    pub fn iter(&self, coord: SpawnCoord) -> LootIter<'_> {
        LootIter::new(self.world_seed, self.table, self.save_flags.as_ref(), coord)
    }

    pub fn spawn(&self, coord: SpawnCoord) -> Vec<Item> {
        self.iter(coord).collect()
    }
}
