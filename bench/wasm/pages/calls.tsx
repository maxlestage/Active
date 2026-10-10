// Micro-benchmarks: JS vs raw wasm vs wasm-bindgen, all in the same page.
import { wasm } from "active-react";

type Raw = { add(a: number, b: number): number; fib(n: number): number };
const raw = wasm<Raw>("/raw.wasm");
const bg = wasm(() => import("../../.build/wasm/bindgen.js"), { url: "/bindgen_bg.wasm" });

const js = {
  add: (a: number, b: number) => (a + b) | 0,
  fib: (n: number): number => (n < 2 ? n : (js.fib(n - 1) + js.fib(n - 2)) >>> 0),
  greet: (name: string) => `Bonjour ${name} !`,
};

function time(fn: () => unknown, runs = 7) {
  fn();
  fn();
  const out: number[] = [];
  for (let k = 0; k < runs; k++) {
    const t0 = performance.now();
    fn();
    out.push(performance.now() - t0);
  }
  return out.sort((a, b) => a - b)[runs >> 1]!;
}

// One loop per variant: a shared loop would make the call site polymorphic and skew results.
let R: Raw;
let B: Awaited<ReturnType<typeof bg.load>>;
const N = 10_000_000;
const G = 300_000;
const NAMES = Array.from({ length: 16 }, (_, i) => `Rust ${i}`);
function addJs() {
  let s = 0;
  for (let i = 0; i < N; i++) s = js.add(s, i);
  return s;
}
function addRaw() {
  let s = 0;
  for (let i = 0; i < N; i++) s = R.add(s, i);
  return s;
}
function addBindgen() {
  let s = 0;
  for (let i = 0; i < N; i++) s = B.add(s, i);
  return s;
}
function greetJs() {
  let l = 0;
  for (let i = 0; i < G; i++) l += js.greet(NAMES[i & 15]!).length;
  return l;
}
function greetBindgen() {
  let l = 0;
  for (let i = 0; i < G; i++) l += B.greet(NAMES[i & 15]!).length;
  return l;
}

(window as any).run = async () => {
  [R, B] = await Promise.all([raw.load(), bg.load()]);
  if (R.fib(25) !== js.fib(25) || B.fib(25) !== js.fib(25) || B.greet("x") !== js.greet("x")) throw new Error("mismatch");
  if (addJs() !== addRaw() || addRaw() !== addBindgen()) throw new Error("mismatch");
  return {
    addNs: { js: (time(addJs) * 1e6) / N, raw: (time(addRaw) * 1e6) / N, bindgen: (time(addBindgen) * 1e6) / N },
    fib30Ms: { js: time(() => js.fib(30)), raw: time(() => R.fib(30)), bindgen: time(() => B.fib(30)) },
    greetNs: { js: (time(greetJs) * 1e6) / G, bindgen: (time(greetBindgen) * 1e6) / G },
  };
};
