use crate::data::{
    Material, POTION_LIQUIDS, POTION_MATERIALS_MAGIC, POTION_MATERIALS_SECRET,
    POTION_MATERIALS_STANDARD, POTION_SANDS,
};
use crate::rng::NollaPrng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionKind {
    StandardOrMagic = 0,
    Secret = 1,
    LiquidOrSand = 2,
}

pub fn create_potion(x: f64, y: f64, seed: u32, kind: PotionKind) -> Material {
    let mut rnd = NollaPrng::new(seed);
    rnd.set_random_seed(x - 4.5, y - 4.0);
    match kind {
        PotionKind::StandardOrMagic => {
            if rnd.random_i32_inclusive(0, 100) <= 75 {
                if rnd.random_i32_inclusive(0, 100000) <= 50 {
                    Material::MagicLiquidHpRegeneration
                } else if rnd.random_i32_inclusive(200, 100000) <= 250 {
                    Material::PurifyingPowder
                } else if rnd.random_i32_inclusive(250, 100000) <= 500 {
                    Material::MagicLiquidWeakness
                } else {
                    POTION_MATERIALS_MAGIC[rnd
                        .random_i32_inclusive(0, POTION_MATERIALS_MAGIC.len() as i32 - 1)
                        as usize]
                }
            } else {
                POTION_MATERIALS_STANDARD[rnd
                    .random_i32_inclusive(0, POTION_MATERIALS_STANDARD.len() as i32 - 1)
                    as usize]
            }
        }
        PotionKind::Secret => {
            POTION_MATERIALS_SECRET
                [rnd.random_i32_inclusive(0, POTION_MATERIALS_SECRET.len() as i32 - 1) as usize]
        }
        PotionKind::LiquidOrSand => {
            if rnd.random_i32_inclusive(0, 100) <= 50 {
                POTION_LIQUIDS
                    [rnd.random_i32_inclusive(0, POTION_LIQUIDS.len() as i32 - 1) as usize]
            } else {
                POTION_SANDS[rnd.random_i32_inclusive(0, POTION_SANDS.len() as i32 - 1) as usize]
            }
        }
    }
}
