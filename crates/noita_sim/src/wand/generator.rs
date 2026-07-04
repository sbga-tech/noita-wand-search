use super::{get_wand_sprite, get_wand_unlocked};
use crate::types::{SaveFlags, Wand};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WandGenerator {
    pub cost: i32,
    pub level: i32,
    pub forced_nonshuffle: bool,
    pub x_offset: f64,
    pub y_offset: f64,
}

impl WandGenerator {
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

    pub(crate) fn spawn_wand(
        &self,
        world_seed: u32,
        x: i32,
        y: i32,
        save_flags: Option<&SaveFlags>,
    ) -> Wand {
        get_wand_unlocked(
            world_seed,
            x as f64 + self.x_offset,
            y as f64 + self.y_offset,
            self.cost,
            self.level,
            self.forced_nonshuffle,
            save_flags,
        )
    }

    pub(crate) fn wand_sprite(&self, world_seed: u32, x: i32, y: i32) -> usize {
        get_wand_sprite(
            world_seed,
            x as f64 + self.x_offset,
            y as f64 + self.y_offset,
            self.cost,
            self.level,
            self.forced_nonshuffle,
        )
    }
}
