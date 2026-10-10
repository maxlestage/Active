import { wasm } from "active-react";
import url from "./pkg/hello_bg.wasm?url";

// Declared once, loaded on first use. `url` lets SSR emit <link rel="preload"> for the binary.
export const hello = wasm(() => import("./pkg/hello.js"), { url });
