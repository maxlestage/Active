use wasm_bindgen::prelude::*;

/// Number of primes below `n` (sieve of Eratosthenes).
#[wasm_bindgen]
pub fn count_primes(n: u32) -> u32 {
    let n = n as usize;
    if n < 2 {
        return 0;
    }
    let mut sieve = vec![true; n];
    sieve[0] = false;
    sieve[1] = false;
    let mut i = 2;
    while i * i < n {
        if sieve[i] {
            let mut j = i * i;
            while j < n {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    sieve.iter().filter(|&&p| p).count() as u32
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Bonjour {name}, ici Rust 🦀")
}
