// Namespace imports: React's Server Components build has no hooks, and a named import would
// make `import { Seo } from "active-react"` fail there.
import * as React from "react";
import * as ReactDOM from "react-dom";

export interface Wasm<T> {
  /** URL of the `.wasm` binary when known; preloaded from `<head>` by `useWasm`. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, `undefined` before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies `listener` when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}

/**
 * Declares a WebAssembly module. Nothing is fetched until `load()` or `useWasm()`.
 *
 * - `wasm(() => import("./pkg/my_crate.js"), { url })` — wasm-bindgen / wasm-pack output
 *   (`--target web` is initialised automatically, `--target bundler` is used as is).
 * - `wasm<Exports>("/my_crate.wasm", { imports })` — raw `extern "C"` module, no JS glue.
 */
export function wasm<T>(source: () => Promise<T>, options?: { url?: string }): Wasm<T>;
export function wasm<T = WebAssembly.Exports>(
  source: string | URL,
  options?: { imports?: WebAssembly.Imports },
): Wasm<T>;
export function wasm<T>(
  source: string | URL | (() => Promise<T>),
  options: { url?: string; imports?: WebAssembly.Imports } = {},
): Wasm<T> {
  const url = typeof source === "function" ? options.url : String(source);
  const start: () => Promise<T> =
    typeof source === "function"
      ? () =>
          source().then(async (m: any) => {
            // wasm-bindgen `--target web` exports an async `default` init function.
            if (typeof m.default === "function") await m.default(url ? { module_or_path: url } : undefined);
            return m;
          })
      : () => instantiate(url!, options.imports) as Promise<T>;

  let value: T | undefined;
  let error: unknown;
  let failed = false;
  let promise: Promise<T> | undefined;
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((l) => l());

  const load = () => {
    if (!promise) {
      failed = false;
      promise = start().then(
        (v) => {
          value = v;
          notify();
          return v;
        },
        (e) => {
          error = e;
          failed = true;
          promise = undefined;
          notify();
          throw e;
        },
      );
    }
    return promise;
  };

  return {
    url,
    load,
    get() {
      if (failed) throw error;
      return value;
    },
    subscribe(listener) {
      listeners.add(listener);
      if (value === undefined && !failed) load().catch(noop);
      return () => void listeners.delete(listener);
    },
  };
}

async function instantiate(url: string, imports: WebAssembly.Imports = {}): Promise<WebAssembly.Exports> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`wasm: ${url} responded ${res.status}`);
  const { instance } = res.headers.get("content-type")?.startsWith("application/wasm")
    ? await WebAssembly.instantiateStreaming(res, imports)
    : await WebAssembly.instantiate(await res.arrayBuffer(), imports);
  return instance.exports;
}

const noop = () => {};
const PRELOAD = { as: "fetch", crossOrigin: "anonymous", type: "application/wasm" } as const;

/**
 * Returns the module exports, or `undefined` while loading (always `undefined` during SSR
 * and hydration, so markup matches). Errors are thrown to the nearest error boundary.
 * When the URL is known, the binary is preloaded from `<head>` — including in SSR output.
 * Prefer Suspense? Use React's `use(mod.load())` in a client-only component.
 */
export function useWasm<T>(mod: Wasm<T>): T | undefined {
  const value = React.useSyncExternalStore(mod.subscribe, mod.get, noop as () => undefined);
  if (value === undefined && mod.url) ReactDOM.preload(mod.url, PRELOAD);
  return value;
}
