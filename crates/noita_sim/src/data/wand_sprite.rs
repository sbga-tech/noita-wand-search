#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WandSprite {
    pub name: &'static str,
    pub file_num: i32,
    pub grip_x: i8,
    pub grip_y: i8,
    pub tip_x: i8,
    pub tip_y: i8,
    pub fire_rate_wait: i8,
    pub actions_per_round: i8,
    pub shuffle_deck_when_empty: bool,
    pub deck_capacity: i8,
    pub spread_degrees: i8,
    pub reload_time: i8,
}
