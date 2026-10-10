import { wasm } from "active-react";
import url from "./wasm/active_site_bg.wasm?url";

// Rust crate in site/wasm (built with `npm run wasm`): hero dust + SEO playground.
export const site = wasm(() => import("./wasm/active_site.js"), { url });
