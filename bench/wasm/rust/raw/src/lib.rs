//! Plain `extern "C"` exports: no wasm-bindgen, no JS glue.

#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn fib(n: u32) -> u32 {
    if n < 2 { n } else { fib(n - 1).wrapping_add(fib(n - 2)) }
}
