import { useWasm, wasm } from "active-react";
import { Ready, mount } from "./ready";

const math = wasm(() => import("../../.build/wasm/bindgen.js"), { url: "/bindgen_bg.wasm" });
mount(() => <Ready value={useWasm(math)?.greet("Rust")} />);
