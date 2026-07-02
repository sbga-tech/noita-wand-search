use super::Spell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpellProb {
    pub p: f64,
    pub spell: Spell,
}
