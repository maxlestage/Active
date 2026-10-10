import { useLayoutEffect } from "react";
import { createRoot } from "react-dom/client";

// Records the moment the wasm result is on screen (ms since navigation start).
export function Ready({ value }: { value?: unknown }) {
  useLayoutEffect(() => {
    if (value !== undefined) (window as any).readyAt = performance.now();
  }, [value]);
  return <p id="out">{value === undefined ? "…" : String(value)}</p>;
}

export const mount = (App: () => React.ReactNode) => createRoot(document.getElementById("root")!).render(<App />);
