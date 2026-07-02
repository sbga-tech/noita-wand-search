use crate::data::Spell;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WandStat {
    Capacity,
    Multicast,
    CastDelay,
    Reload,
    MaxMana,
    ManaRegen,
    Spread,
    Speed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Wand {
    pub capacity: i32,
    pub multicast: i32,
    pub mana: i32,
    pub regen: i32,
    pub delay: f64,
    pub reload: f64,
    pub speed: f32,
    pub spread: i32,
    pub shuffle: bool,
    pub always_cast: Spell,
    pub sprite: usize,
    pub spells: Vec<Spell>,
}

impl Default for Wand {
    fn default() -> Self {
        Self {
            capacity: 0,
            multicast: 0,
            mana: 0,
            regen: 0,
            delay: 0.0,
            reload: 0.0,
            speed: 0.0,
            spread: 0,
            shuffle: true,
            always_cast: Spell::None,
            sprite: 0,
            spells: Vec::new(),
        }
    }
}
