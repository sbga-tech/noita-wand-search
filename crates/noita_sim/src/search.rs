use rayon::prelude::*;

use crate::filters::{wand_matches_filters, WandFilterSet};
use crate::loot::{
    round_rng_pos, LootSpawner, SpawnCoord, GREAT_CHEST_LOOT_TABLE, TAIKASAUVA_LOOT_TABLE,
    TINY_DROP_LOOT_TABLE,
};
use crate::types::Wand;
use crate::wandgen::SaveFlags;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SearchMode {
    EoeWand,
    TaikasauvaWand,
    TinyDropWand,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchRequest {
    pub seed: u32,
    pub ng: u32,
    pub start_x: f64,
    pub start_y: f64,
    pub mode: SearchMode,
    pub wand_filters: WandFilterSet,
    pub unlock_flags: Option<Vec<String>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchProgress {
    pub x: f64,
    pub y: f64,
    pub searched_pixels: u32,
}

pub struct SearchState {
    x_center: f64,
    y_center: f64,
    x_off: f64,
    y_off: f64,
    x_step_mult: f64,
    y_step_mult: f64,
    x_step: f64,
    y_step: f64,
    searched_pixels: u32,
    last_x: f64,
    last_y: f64,
    mode: SearchMode,
    wand_filters: WandFilterSet,
    loot_spawner: LootSpawner,
}

#[derive(Clone, Debug)]
pub enum SearchHit {
    Wand {
        x: f64,
        y: f64,
        searched_pixels: u32,
        wand: Wand,
    },
}

#[derive(Clone, Copy, Debug)]
struct SpiralCandidate {
    x: i32,
    y: i32,
    searched_pixels: u32,
    state: SpiralState,
}

#[derive(Clone, Copy, Debug)]
struct SpiralState {
    last_x: f64,
    last_y: f64,
    x_off: f64,
    y_off: f64,
    x_step: f64,
    y_step: f64,
}

impl SearchState {
    pub fn new(request: SearchRequest) -> Self {
        let SearchRequest {
            seed,
            ng,
            start_x,
            start_y,
            mode,
            wand_filters,
            unlock_flags,
        } = request;
        let mode_is_chest = matches!(mode, SearchMode::EoeWand);
        let x_step_mult = if mode_is_chest && start_x.abs() >= 10_000_000.0 {
            100.0
        } else if mode_is_chest && start_x.abs() >= 1_000_000.0 {
            10.0
        } else {
            1.0
        };
        let y_step_mult = if mode_is_chest && start_y.abs() >= 10_000_000.0 {
            100.0
        } else if mode_is_chest && start_y.abs() >= 1_000_000.0 {
            10.0
        } else {
            1.0
        };
        let effective_seed = seed.wrapping_add(ng);
        let loot_table = match mode {
            SearchMode::EoeWand => &GREAT_CHEST_LOOT_TABLE,
            SearchMode::TaikasauvaWand => &TAIKASAUVA_LOOT_TABLE,
            SearchMode::TinyDropWand => &TINY_DROP_LOOT_TABLE,
        };
        let save_flags = match (mode, unlock_flags) {
            (SearchMode::EoeWand, Some(flags)) => Some(SaveFlags::new(flags)),
            (SearchMode::EoeWand, None) => Some(SaveFlags::new(Vec::new())),
            (_, Some(flags)) => Some(SaveFlags::new(flags)),
            (_, None) => None,
        };
        let loot_spawner = LootSpawner::new(effective_seed, loot_table, save_flags);
        Self {
            x_center: start_x,
            y_center: start_y,
            x_off: 0.0,
            y_off: 0.0,
            x_step_mult,
            y_step_mult,
            x_step: 1.0,
            y_step: 0.0,
            loot_spawner,
            searched_pixels: 0,
            last_x: start_x,
            last_y: start_y,
            mode,
            wand_filters,
        }
    }

    pub fn step(&mut self, max_iterations: u32) -> Option<SearchHit> {
        if max_iterations > 1 {
            return self.step_parallel(max_iterations);
        }

        self.step_serial(max_iterations)
    }

    fn step_serial(&mut self, max_iterations: u32) -> Option<SearchHit> {
        for _ in 0..max_iterations {
            let candidate = self.next_candidate();
            if let Some(hit) =
                self.check_candidate(candidate.x, candidate.y, candidate.searched_pixels)
            {
                return Some(hit);
            }
            self.advance_spiral();
        }
        None
    }

    fn step_parallel(&mut self, max_iterations: u32) -> Option<SearchHit> {
        let mut candidates = Vec::with_capacity(max_iterations as usize);
        for _ in 0..max_iterations {
            candidates.push(self.next_candidate());
            self.advance_spiral();
        }

        let hit = candidates.par_iter().find_map_first(|candidate| {
            self.check_candidate(candidate.x, candidate.y, candidate.searched_pixels)
                .map(|hit| (*candidate, hit))
        });
        if let Some((candidate, hit)) = hit {
            self.restore_candidate(candidate);
            return Some(hit);
        }
        None
    }

    fn next_candidate(&mut self) -> SpiralCandidate {
        self.searched_pixels = self.searched_pixels.wrapping_add(1);
        let mut x_seed = (self.x_center + self.x_off).floor();
        let mut y_seed = (self.y_center + self.y_off).floor();
        if matches!(self.mode, SearchMode::EoeWand) {
            x_seed = round_rng_pos(x_seed as i32) as f64;
            y_seed = round_rng_pos(y_seed as i32) as f64;
        }
        self.last_x = x_seed;
        self.last_y = y_seed;
        SpiralCandidate {
            x: x_seed as i32,
            y: y_seed as i32,
            searched_pixels: self.searched_pixels,
            state: SpiralState {
                last_x: self.last_x,
                last_y: self.last_y,
                x_off: self.x_off,
                y_off: self.y_off,
                x_step: self.x_step,
                y_step: self.y_step,
            },
        }
    }

    fn restore_candidate(&mut self, candidate: SpiralCandidate) {
        self.searched_pixels = candidate.searched_pixels;
        self.last_x = candidate.state.last_x;
        self.last_y = candidate.state.last_y;
        self.x_off = candidate.state.x_off;
        self.y_off = candidate.state.y_off;
        self.x_step = candidate.state.x_step;
        self.y_step = candidate.state.y_step;
    }

    fn advance_spiral(&mut self) {
        const EPSILON: f64 = 0.1;
        self.x_off += self.x_step * self.x_step_mult;
        self.y_off += self.y_step * self.y_step_mult;
        if (((self.x_off / self.x_step_mult).abs() - (self.y_off / self.y_step_mult).abs()).abs()
            < EPSILON
            && self.x_step <= EPSILON)
            || (((self.x_off / self.x_step_mult) - 1.0 + (self.y_off / self.y_step_mult)).abs()
                < EPSILON
                && self.x_step > EPSILON)
        {
            let temp = self.x_step;
            self.x_step = -self.y_step;
            self.y_step = temp;
        }
    }

    fn check_candidate(&self, x: i32, y: i32, searched_pixels: u32) -> Option<SearchHit> {
        if matches!(self.mode, SearchMode::EoeWand) && x == 0 && y == 0 {
            return None;
        }

        self.loot_spawner
            .iter(SpawnCoord { x, y })
            .find_map(|item| match item {
                crate::loot::Item::Wand(wand)
                    if wand_matches_filters(&wand, &self.wand_filters) =>
                {
                    Some(wand)
                }
                _ => None,
            })
            .map(|wand| SearchHit::Wand {
                x: x as f64,
                y: y as f64,
                searched_pixels,
                wand,
            })
    }

    pub fn progress(&self) -> SearchProgress {
        SearchProgress {
            x: self.last_x,
            y: self.last_y,
            searched_pixels: self.searched_pixels,
        }
    }
}
