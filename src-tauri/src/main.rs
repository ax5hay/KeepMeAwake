//! Binary entry point. All logic lives in the library crate so it can be tested.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    keepmeawake_lib::run();
}
