use crate::data::Material;
use crate::potion::{PotionGenerator, PotionKind};
use crate::rng::NollaPrng;
use crate::types::{SaveFlags, Wand};
use crate::wand::WandGenerator;

use tinyvec::{tiny_vec, TinyVec};

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

macro_rules! bitflags {
    (
        $vis:vis struct $flags:ident($repr:ty) {
            $($flag:ident = $value:expr),+ $(,)?
        }
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis struct $flags($repr);

        impl $flags {
            $(
                $vis const $flag: Self = Self($value);
            )+
            $vis const ALL: Self = Self(0 $(| Self::$flag.0)+);

            fn contains(self, flag: Self) -> bool {
                self.0 & flag.0 == flag.0
            }

            fn intersects(self, flag: Self) -> bool {
                self.0 & flag.0 != 0
            }
        }
    };
}

bitflags! {
    pub(crate) struct ItemFlags(u8) {
        WAND = 1u8 << 0,
        MATERIAL = 1u8 << 1,
        PLACEHOLDER = 1u8 << 2,
    }
}

#[derive(Clone, Copy)]
struct SpawnContext<'a> {
    world_seed: u32,
    coord: SpawnCoord,
    save_flags: Option<&'a SaveFlags>,
}

trait GenerateItem {
    fn item_flags(&self) -> ItemFlags;
    fn next_item(&self, index: usize, context: SpawnContext<'_>) -> Option<Item>;
}

impl GenerateItem for WandGenerator {
    fn item_flags(&self) -> ItemFlags {
        ItemFlags::WAND
    }

    fn next_item(&self, index: usize, context: SpawnContext<'_>) -> Option<Item> {
        (index == 0).then(|| {
            Item::Wand(self.spawn_wand(
                context.world_seed,
                context.coord.x,
                context.coord.y,
                context.save_flags,
            ))
        })
    }
}

impl GenerateItem for PotionGenerator {
    fn item_flags(&self) -> ItemFlags {
        ItemFlags::MATERIAL
    }

    fn next_item(&self, index: usize, context: SpawnContext<'_>) -> Option<Item> {
        self.create_material(
            index,
            context.coord.x as f64,
            context.coord.y as f64,
            context.world_seed,
        )
        .map(Item::Material)
    }
}

enum ItemGenerator {
    Wand(WandGenerator),
    Potions(PotionGenerator),
}

impl GenerateItem for ItemGenerator {
    fn item_flags(&self) -> ItemFlags {
        match self {
            Self::Wand(generator) => generator.item_flags(),
            Self::Potions(generator) => generator.item_flags(),
        }
    }

    fn next_item(&self, index: usize, context: SpawnContext<'_>) -> Option<Item> {
        match self {
            Self::Wand(generator) => generator.next_item(index, context),
            Self::Potions(generator) => generator.next_item(index, context),
        }
    }
}

enum LootNode {
    Spawner(ItemGenerator),
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

const EMPTY_LOOT_TABLE: LootTable = LootTable {
    min_roll: 0,
    max_roll: 0,
    entries: &[],
};

impl Default for LootTable {
    fn default() -> Self {
        EMPTY_LOOT_TABLE
    }
}

impl Default for &'static LootTable {
    fn default() -> Self {
        &EMPTY_LOOT_TABLE
    }
}

impl LootTable {
    fn roll(&'static self, random: &mut NollaPrng) -> &'static LootNode {
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
        const _: () = assert!(wand_generator_table_is_valid(TABLE));
        wand_generator_weights_from_table::<{ TABLE.entries.len() }>(TABLE)
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
            node: LootNode::Spawner(ItemGenerator::Potions(PotionGenerator::new(
                &POTION_STANDARD_STANDARD_SECRET,
            ))),
        },
        {
            threshold: 100,
            node: LootNode::Spawner(ItemGenerator::Potions(PotionGenerator::new(
                &POTION_SECRET_SECRET_RANDOM_MATERIAL,
            ))),
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
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(80, 4, false))),
        },
        {
            threshold: 50,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(80, 4, true))),
        },
        {
            threshold: 75,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(100, 5, false))),
        },
        {
            threshold: 90,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(100, 5, true))),
        },
        {
            threshold: 96,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(120, 6, false))),
        },
        {
            threshold: 98,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(120, 6, true))),
        },
        {
            threshold: 99,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(120, 6, false))),
        },
        {
            threshold: 100,
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(200, 11, false))),
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
            node: LootNode::Spawner(ItemGenerator::Wand(WandGenerator::new(60, 3, false))),
        },
    ]
});

pub const TINY_DROP_LOOT_TABLE: LootTable = loot_table!({
    min_roll: 0,
    max_roll: 0,
    [
        {
            threshold: 0,
            node: LootNode::Spawner(ItemGenerator::Wand(
                WandGenerator::new(180, 11, true).with_offset(16.0, 0.0),
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

const fn wand_generator_from_node(node: &LootNode) -> Option<WandGenerator> {
    match node {
        LootNode::Spawner(ItemGenerator::Wand(generator)) => Some(*generator),
        _ => None,
    }
}

const fn wand_generator_table_is_valid(table: LootTable) -> bool {
    if !loot_table_is_valid(table) {
        return false;
    }
    let entries = table.entries;
    let mut i = 0;
    while i < entries.len() {
        if wand_generator_from_node(&entries[i].node).is_none() {
            return false;
        }
        i += 1;
    }
    true
}

const fn wand_generator_weights_from_table<const N: usize>(
    table: LootTable,
) -> [(WandGenerator, f64); N] {
    let entries = table.entries;
    let denominator = (table.max_roll - table.min_roll + 1) as f64;
    let mut out = [(WandGenerator::new(0, 0, false), 0.0); N];
    let mut previous_threshold = table.min_roll - 1;
    let mut i = 0;
    while i < N {
        let count = entries[i].threshold - previous_threshold;
        let generator = match wand_generator_from_node(&entries[i].node) {
            Some(generator) => generator,
            None => WandGenerator::new(0, 0, false),
        };
        out[i] = (generator, count as f64 / denominator);
        previous_threshold = entries[i].threshold;
        i += 1;
    }
    out
}

pub const GREAT_CHEST_WAND_GENERATOR_WEIGHTS: [(WandGenerator, f64);
    GREAT_CHEST_WAND_TABLE.entries.len()] = loot_table_weight!(GREAT_CHEST_WAND_TABLE);

pub fn great_chest_wand_generator_weights() -> &'static [(WandGenerator, f64)] {
    &GREAT_CHEST_WAND_GENERATOR_WEIGHTS
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
    pub(crate) world_seed: u32,
    pub(crate) table: &'static LootTable,
    pub(crate) save_flags: Option<SaveFlags>,
}

const INLINE_PENDING_TABLES: usize = 8;

fn push_pending_table(
    pending_tables: &mut TinyVec<[(&'static LootTable, u32); INLINE_PENDING_TABLES]>,
    table: &'static LootTable,
    count: u32,
) {
    if count != 0 {
        pending_tables.push((table, count));
    }
}

fn pop_pending_table(
    pending_tables: &mut TinyVec<[(&'static LootTable, u32); INLINE_PENDING_TABLES]>,
) -> Option<&'static LootTable> {
    let (table, count) = pending_tables.last_mut()?;
    let table = *table;
    *count -= 1;
    if *count == 0 {
        pending_tables.pop();
    }
    Some(table)
}

fn loot_random(world_seed: u32, table: &'static LootTable, coord: SpawnCoord) -> NollaPrng {
    let mut random = NollaPrng::new(world_seed);
    if core::ptr::addr_eq(table, &GREAT_CHEST_LOOT_TABLE) {
        random.set_random_seed_int(round_rng_pos(coord.x), coord.y);
    } else {
        random.set_random_seed(coord.x as f64, coord.y as f64);
    }
    random
}

pub struct LootIter<'a> {
    world_seed: u32,
    coord: SpawnCoord,
    save_flags: Option<&'a SaveFlags>,
    random: NollaPrng,
    pending_tables: TinyVec<[(&'static LootTable, u32); INLINE_PENDING_TABLES]>,
    active_generator: Option<&'static ItemGenerator>,
    active_generator_index: usize,
    item_flags: ItemFlags,
}

impl<'a> LootIter<'a> {
    fn new(
        world_seed: u32,
        table: &'static LootTable,
        save_flags: Option<&'a SaveFlags>,
        coord: SpawnCoord,
        item_flags: ItemFlags,
    ) -> Self {
        let random = loot_random(world_seed, table, coord);

        Self {
            world_seed,
            coord,
            save_flags,
            random,
            pending_tables: tiny_vec!([(&'static LootTable, u32); INLINE_PENDING_TABLES] => (table, 1)),
            active_generator: None,
            active_generator_index: 0,
            item_flags,
        }
    }
}

impl Iterator for LootIter<'_> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        let context = SpawnContext {
            world_seed: self.world_seed,
            coord: self.coord,
            save_flags: self.save_flags,
        };
        loop {
            if let Some(generator) = self.active_generator {
                if let Some(item) = generator.next_item(self.active_generator_index, context) {
                    self.active_generator_index += 1;
                    return Some(item);
                }
                self.active_generator = None;
                self.active_generator_index = 0;
            }

            let table = pop_pending_table(&mut self.pending_tables)?;
            match table.roll(&mut self.random) {
                LootNode::Spawner(spawner) => {
                    if self.item_flags.intersects(spawner.item_flags()) {
                        if let Some(item) = spawner.next_item(0, context) {
                            self.active_generator = Some(spawner);
                            self.active_generator_index = 1;
                            return Some(item);
                        }
                    }
                }
                LootNode::ItemPlaceholder(name) => {
                    if self.item_flags.contains(ItemFlags::PLACEHOLDER) {
                        return Some(Item::Placeholder(name));
                    }
                }
                LootNode::Table(nested) => push_pending_table(&mut self.pending_tables, nested, 1),
                LootNode::Reroll(times) => {
                    push_pending_table(&mut self.pending_tables, table, *times)
                }
            }
        }
    }
}

pub fn find_wand_sprite(
    world_seed: u32,
    table: &'static LootTable,
    save_flags: Option<&SaveFlags>,
    coord: SpawnCoord,
    target: &Wand,
) -> Option<usize> {
    let mut random = loot_random(world_seed, table, coord);
    let mut pending_tables =
        tiny_vec!([(&'static LootTable, u32); INLINE_PENDING_TABLES] => (table, 1));
    while let Some(table) = pop_pending_table(&mut pending_tables) {
        match table.roll(&mut random) {
            LootNode::Spawner(ItemGenerator::Wand(generator)) => {
                let wand = generator.spawn_wand(world_seed, coord.x, coord.y, save_flags);
                if &wand == target {
                    return Some(generator.wand_sprite(world_seed, coord.x, coord.y, save_flags));
                }
            }
            LootNode::Spawner(ItemGenerator::Potions(_)) | LootNode::ItemPlaceholder(_) => {}
            LootNode::Table(nested) => push_pending_table(&mut pending_tables, nested, 1),
            LootNode::Reroll(times) => push_pending_table(&mut pending_tables, table, *times),
        }
    }
    None
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
        self.iter_with_flags(coord, ItemFlags::ALL)
    }

    pub(crate) fn iter_with_flags(&self, coord: SpawnCoord, item_flags: ItemFlags) -> LootIter<'_> {
        LootIter::new(
            self.world_seed,
            self.table,
            self.save_flags.as_ref(),
            coord,
            item_flags,
        )
    }

    pub fn spawn(&self, coord: SpawnCoord) -> Vec<Item> {
        self.iter(coord).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_coords() -> impl Iterator<Item = SpawnCoord> {
        (0..256).map(|index| SpawnCoord {
            x: index * 37 - 2_000,
            y: index * 53 + 10,
        })
    }

    #[test]
    fn generated_wand_sprite_can_be_recovered_lazily() {
        let spawner = LootSpawner::new(123_456, &GREAT_CHEST_LOOT_TABLE, None);
        let mut saw_wand = false;

        for coord in sample_coords() {
            for item in spawner.iter_with_flags(coord, ItemFlags::WAND) {
                if let Item::Wand(wand) = item {
                    let sprite = find_wand_sprite(
                        spawner.world_seed,
                        spawner.table,
                        spawner.save_flags.as_ref(),
                        coord,
                        &wand,
                    );
                    assert!(sprite.is_some(), "generated wand should recover a sprite");
                    saw_wand = true;
                }
            }
        }

        assert!(
            saw_wand,
            "sample coordinates should include at least one wand"
        );
    }

    #[test]
    fn pruned_wand_projection_matches_full_iterator() {
        let spawner = LootSpawner::new(123_456, &GREAT_CHEST_LOOT_TABLE, None);
        let mut saw_wand = false;

        for coord in sample_coords() {
            let from_iter = spawner.iter(coord).find_map(|item| match item {
                Item::Wand(wand) => Some(wand),
                Item::Material(_) | Item::Placeholder(_) => None,
            });
            let from_pruned = spawner
                .iter_with_flags(coord, ItemFlags::WAND)
                .find_map(|item| match item {
                    Item::Wand(wand) => Some(wand),
                    Item::Material(_) | Item::Placeholder(_) => {
                        panic!("wand-only projection yielded a non-wand item")
                    }
                });

            saw_wand |= from_iter.is_some();
            assert_eq!(from_pruned, from_iter);
        }

        assert!(
            saw_wand,
            "sample coordinates should include at least one wand"
        );
    }

    #[test]
    fn all_candidate_projection_matches_public_iterator() {
        let spawner = LootSpawner::new(123_456, &GREAT_CHEST_LOOT_TABLE, None);
        let mut saw_material = false;

        for coord in sample_coords() {
            let from_find_map = spawner.iter_with_flags(coord, ItemFlags::ALL).next();
            let from_iter = spawner.iter(coord).next();

            saw_material |= matches!(from_iter, Some(Item::Material(_)));
            assert_eq!(from_find_map, from_iter);
        }

        assert!(
            saw_material,
            "sample coordinates should cover material item branches"
        );
    }
}
