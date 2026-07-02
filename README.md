# Noita Wand Search

This tool searches wands from various loot with a set of custom filters in the game Noita.

## Local development

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
npm ci --prefix crates/noita_web
git submodule update --init --depth 1 noita-data
cd crates/noita_web && PATH="$PWD/node_modules/.bin:$PATH" trunk serve
```

Open <http://127.0.0.1:3000/>.

## Verification

```sh
cargo fmt -p noita_web --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd crates/noita_web && PATH="$PWD/node_modules/.bin:$PATH" trunk build --release=true
```

## Production deployment

Deploy the static files generated in `crates/noita_web/dist/`.

## Thanks
https://github.com/pudy248/noitaWandAtlas
https://noita.wiki.gg/wiki/Technical:_Noita_PRNG
https://noita.wiki.gg/wiki/Wand_Generation
https://noita.wiki.gg/wiki/Great_Treasure_Chest
