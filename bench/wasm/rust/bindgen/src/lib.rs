//! Same functions through wasm-bindgen, plus a string round-trip.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[wasm_bindgen]
pub fn fib(n: u32) -> u32 {
    if n < 2 { n } else { fib(n - 1).wrapping_add(fib(n - 2)) }
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Bonjour {name} !")
}
