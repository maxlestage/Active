import { expected } from "./shared";

// Runs in its own process per library: warm up, then sample batches for ~2.5 s.
export function measure(ssr: (i: number) => string) {
  const html = ssr(7);
  const head = html.slice(0, html.indexOf("</head>"));
  const e = expected(7);
  const check = {
    title: head.split(`<title`).length - 1 === 1 && head.includes(e.title),
    description: head.includes(e.description),
    canonical: head.includes(e.url),
    jsonLd: html.includes('"@type":"Product"'),
  };
  let i = 0;
  for (const end = performance.now() + 800; performance.now() < end; ) ssr(i++ % 100);
  const samples: number[] = [];
  for (const end = performance.now() + 2500; performance.now() < end; ) {
    const t0 = performance.now();
    for (let k = 0; k < 20; k++) ssr(i++ % 100);
    samples.push((performance.now() - t0) / 20);
  }
  samples.sort((a, b) => a - b);
  const us = samples[samples.length >> 1]! * 1000;
  console.log(JSON.stringify({ us, opsPerSec: 1e6 / us, check, bytes: html.length, head }));
}
