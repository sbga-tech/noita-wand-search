use crate::data::Spell;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

pub const WAND_SPELL_INLINE_CAPACITY: usize = 26;
pub type WandSpells = TinyVec<[Spell; WAND_SPELL_INLINE_CAPACITY]>;

#[derive(Clone, Debug)]
pub struct SaveFlags {
    flags: Vec<String>,
}

impl SaveFlags {
    pub fn new(flags: Vec<String>) -> Self {
        Self { flags }
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|candidate| candidate == flag)
    }

    pub fn is_spell_unlocked(&self, spell: Spell) -> bool {
        match spell.unlock_flag() {
            None => true,
            Some(flag) => self.has_flag(flag),
        }
    }
}

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

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub spells: WandSpells,
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
            spells: TinyVec::new(),
        }
    }
}
