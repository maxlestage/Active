// Hand-written baseline for wasm-bindgen glue.
import { useEffect, useState } from "react";
import init, { greet } from "../../.build/wasm/bindgen.js";
import { Ready, mount } from "./ready";

mount(() => {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    init({ module_or_path: "/bindgen_bg.wasm" }).then(() => setReady(true));
  }, []);
  return <Ready value={ready ? greet("Rust") : undefined} />;
});
