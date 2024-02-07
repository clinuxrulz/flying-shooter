use wasm_bindgen::prelude::wasm_bindgen;

mod args;
mod components;
mod input;
mod game;
mod fps_plugin;
mod pbr_material;

#[wasm_bindgen]
pub fn run_game() {
    game::run_game();
}