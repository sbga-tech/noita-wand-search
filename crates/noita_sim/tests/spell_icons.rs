use noita_sim::Spell;

/// Every spell except `None` must expose a non-empty gun-actions icon basename,
/// so the wand card never renders a broken image.
#[test]
fn every_spell_has_an_icon_except_none() {
    assert_eq!(Spell::None.icon(), "");
    for spell in Spell::ALL {
        if spell == Spell::None {
            continue;
        }
        assert!(
            !spell.icon().is_empty(),
            "{spell:?} ({}) has no icon",
            spell.id()
        );
    }
}

/// The icon basename comes from the lua `sprite` path, which frequently differs
/// from the lowercased id. Pin the divergent cases so regressions surface.
#[test]
fn icon_uses_sprite_basename_not_id() {
    assert_eq!(Spell::CircleshotA.icon(), "phantomshot_a");
    assert_eq!(Spell::CircleshotB.icon(), "phantomshot_b");
    assert_eq!(Spell::Soilball.icon(), "soil");
    assert_eq!(Spell::ExplodingDucks.icon(), "duck_2");
    assert_eq!(Spell::WormShot.icon(), "worm");
    assert_eq!(Spell::ManaReduce.icon(), "mana");
    // A straightforward id==basename case still resolves.
    assert_eq!(Spell::Bomb.icon(), "bomb");
}
