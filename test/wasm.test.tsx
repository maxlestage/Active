// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { Component, act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { useWasm, wasm } from "../src/index.js";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;

// (module (func (export "add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))
const ADD = new Uint8Array([
  0, 97, 115, 109, 1, 0, 0, 0, 1, 7, 1, 96, 2, 127, 127, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, 97, 100, 100, 0, 0, 10, 9, 1,
  7, 0, 32, 0, 32, 1, 106, 11,
]);
type Add = { add(a: number, b: number): number };

const serve = (status = 200, type = "application/wasm") =>
  vi.spyOn(globalThis, "fetch").mockImplementation(
    async () => new Response(status === 200 ? ADD : null, { status, headers: { "content-type": type } }),
  );

async function mount(node: ReactNode) {
  const el = document.body.appendChild(document.createElement("div"));
  await act(() => createRoot(el).render(node));
  return el;
}
const settle = () => act(() => new Promise((r) => setTimeout(r, 0)));

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
  document.head.innerHTML = "";
});

describe("wasm()", () => {
  it("is lazy and loads only once", async () => {
    const fetch = serve();
    const mod = wasm<Add>("/add.wasm");
    expect(fetch).not.toHaveBeenCalled();
    const [a, b] = await Promise.all([mod.load(), mod.load()]);
    expect(a).toBe(b);
    expect(a.add(2, 3)).toBe(5);
    expect(mod.get()).toBe(a);
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("falls back to arrayBuffer when the MIME type is wrong", async () => {
    serve(200, "application/octet-stream");
    expect((await wasm<Add>("/add.wasm").load()).add(40, 2)).toBe(42);
  });

  it("initialises wasm-bindgen glue with the given url", async () => {
    const init = vi.fn(async () => ({}));
    const mod = wasm(async () => ({ default: init, greet: (n: string) => `hi ${n}` }), { url: "/pkg_bg.wasm" });
    const m = await mod.load();
    expect(init).toHaveBeenCalledWith({ module_or_path: "/pkg_bg.wasm" });
    expect(m.greet("rust")).toBe("hi rust");
  });

  it("uses bundler-target modules as is", async () => {
    const m = await wasm(async () => ({ double: (n: number) => n * 2 })).load();
    expect(m.double(4)).toBe(8);
  });

  it("retries after a failure", async () => {
    serve(404);
    const mod = wasm<Add>("/missing.wasm");
    await expect(mod.load()).rejects.toThrow("responded 404");
    expect(() => mod.get()).toThrow("responded 404");
    serve();
    expect((await mod.load()).add(1, 1)).toBe(2);
  });
});

describe("useWasm()", () => {
  it("renders undefined, then the exports", async () => {
    serve();
    const mod = wasm<Add>("/add.wasm");
    const App = () => <p>{useWasm(mod)?.add(20, 22) ?? "loading"}</p>;
    const el = await mount(<App />);
    await settle();
    expect(el.textContent).toBe("42");
    // Already loaded: renders synchronously.
    expect((await mount(<App />)).textContent).toBe("42");
  });

  it("throws load errors to the nearest error boundary", async () => {
    serve(500);
    vi.spyOn(console, "error").mockImplementation(() => {});
    class Boundary extends Component<{ children: ReactNode }, { error?: Error }> {
      state: { error?: Error } = {};
      static getDerivedStateFromError = (error: Error) => ({ error });
      render = () => (this.state.error ? `error: ${this.state.error.message}` : this.props.children);
    }
    const mod = wasm<Add>("/broken.wasm");
    const App = () => <p>{useWasm(mod) ? "ok" : "loading"}</p>;
    const el = await mount(
      <Boundary>
        <App />
      </Boundary>,
    );
    await settle();
    expect(el.textContent).toBe("error: wasm: /broken.wasm responded 500");
  });

  it("server-renders the loading state and preloads the binary", () => {
    const mod = wasm<Add>("/add.wasm");
    const App = () => <p>{useWasm(mod) ? "ready" : "loading"}</p>;
    const html = renderToString(
      <html>
        <head />
        <body>
          <App />
        </body>
      </html>,
    );
    expect(html).toContain('<link rel="preload" href="/add.wasm" as="fetch" crossorigin="" type="application/wasm"/>');
    expect(html).toContain("<p>loading</p>");
  });
});
