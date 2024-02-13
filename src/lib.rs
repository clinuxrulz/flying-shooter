use wasm_bindgen::prelude::wasm_bindgen;

mod args;
mod components;
mod input;
mod game;
mod fps_plugin;
mod math;
mod pbr_material;
mod radar;

#[wasm_bindgen]
pub fn run_game() {
    game::run_game();
}