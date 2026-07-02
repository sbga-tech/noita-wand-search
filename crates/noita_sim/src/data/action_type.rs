use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    num_enum::IntoPrimitive,
    num_enum::TryFromPrimitive,
)]
#[repr(u8)]
pub enum ActionType {
    Projectile,
    StaticProjectile,
    Modifier,
    DrawMany,
    Material,
    Other,
    Utility,
    Passive,
}
