// Hand-written baseline: useState + useEffect + instantiateStreaming.
import { useEffect, useState } from "react";
import { Ready, mount } from "./ready";

mount(() => {
  const [exports, setExports] = useState<any>();
  useEffect(() => {
    WebAssembly.instantiateStreaming(fetch("/raw.wasm")).then((r) => setExports(r.instance.exports));
  }, []);
  return <Ready value={exports?.add(2, 3)} />;
});
