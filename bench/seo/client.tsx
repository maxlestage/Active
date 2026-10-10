import type { ComponentType } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { expected } from "./shared";

// Head is "correct" when title, description and canonical all match page i.
const ok = (i: number) => {
  const e = expected(i);
  return (
    document.title === e.title &&
    document.head.querySelector('meta[name="description"]')?.getAttribute("content") === e.description &&
    document.querySelector('link[rel="canonical"]')?.getAttribute("href") === e.url
  );
};

// Polls with 0-delay macrotasks so libraries that defer to rAF/microtasks get the time they need.
const channel = new MessageChannel();
let poll: (() => void) | undefined;
channel.port1.onmessage = () => poll?.();
const until = (i: number) =>
  new Promise<void>((resolve, reject) => {
    const t0 = performance.now();
    (poll = () =>
      ok(i)
        ? resolve()
        : performance.now() - t0 > 5000
          ? reject(new Error(`head never matched page ${i}: ${document.head.innerHTML.slice(0, 300)}`))
          : channel.port2.postMessage(0))();
  });

export function start(Root: ComponentType<{ i: number }>) {
  const root = createRoot(document.getElementById("root")!);
  const render = (i: number) => flushSync(() => root.render(<Root i={i} />));
  let i = 0;

  render(0);
  until(0).then(
    () => ((window as any).readyAt = performance.now()),
    (e) => ((window as any).readyError = e.message),
  );

  (window as any).bench = {
    async throughput(n: number) {
      const t0 = performance.now();
      for (let k = 0; k < n; k++) render(++i);
      await until(i);
      return performance.now() - t0;
    },
    async latency(n: number) {
      const out: number[] = [];
      for (let k = 0; k < n; k++) {
        const t0 = performance.now();
        render(++i);
        await until(i);
        out.push(performance.now() - t0);
      }
      return out.sort((a, b) => a - b);
    },
    audit() {
      const count = (s: string) => document.querySelectorAll(s).length;
      return {
        title: count("title"),
        description: count('meta[name="description"]'),
        canonical: count('link[rel="canonical"]'),
        ogTitle: count('meta[property="og:title"]'),
        jsonLd: count('script[type="application/ld+json"]'),
        correct: ok(i),
      };
    },
  };
}
