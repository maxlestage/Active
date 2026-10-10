// Server-rendered page; PRELOAD decides whether the binary URL is declared (→ <link rel=preload>).
import { useWasm, wasm } from "active-react";
import { Ready } from "./ready";

declare const PRELOAD: boolean;
const math = wasm(() => import("../../.build/wasm/bindgen.js"), PRELOAD ? { url: "/bindgen_bg.wasm" } : {});
export const App = () => <Ready value={useWasm(math)?.greet("Rust")} />;
