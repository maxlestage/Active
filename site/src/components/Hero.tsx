import { useWasm } from "active-react";
import { useEffect, useRef } from "react";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { GITHUB } from "../seo";
import { site } from "../wasm";
import { Arrow } from "./Logo";
import { Split } from "./Split";

type Site = NonNullable<ReturnType<typeof site.get>>;

/** Points of the word "active" drawn in the display font, in canvas pixels. */
function wordPoints(width: number, height: number, mobile: boolean): Float32Array {
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  // The word floats in the upper part of the hero, above the headline.
  const size = Math.min(width / (mobile ? 3.5 : 3.6), height * (mobile ? 0.2 : 0.3));
  ctx.font = `600 ${size}px "Geist Variable", system-ui, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("active", width / 2, height * (mobile ? 0.25 : 0.28));
  const data = ctx.getImageData(0, 0, width, height).data;
  const step = Math.max(2, Math.round(size / 55));
  const out: number[] = [];
  for (let y = 0; y < height; y += step)
    for (let x = 0; x < width; x += step) if (data[(y * width + x) * 4 + 3]! > 140) out.push(x, y);
  // Shuffle so particles pick targets all over the word.
  for (let i = out.length / 2 - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i * 2], out[j * 2]] = [out[j * 2]!, out[i * 2]!];
    [out[i * 2 + 1], out[j * 2 + 1]] = [out[j * 2 + 1]!, out[i * 2 + 1]!];
  }
  return new Float32Array(out);
}

function animate(canvas: HTMLCanvasElement, rust: Site, hud: HTMLElement | null) {
  const ctx = canvas.getContext("2d")!;
  const reduce = document.documentElement.classList.contains("reduce");
  const dpr = Math.min(devicePixelRatio || 1, 2);
  let w = 0, h = 0, dust: InstanceType<Site["Dust"]> | undefined;
  let pointer = { x: NaN, y: NaN };
  let visible = true, raf = 0, last = performance.now(), fps = 60, lastHud = 0;

  const setup = () => {
    const r = canvas.getBoundingClientRect();
    w = Math.max(1, Math.round(r.width * dpr));
    h = Math.max(1, Math.round(r.height * dpr));
    canvas.width = w;
    canvas.height = h;
    const mobile = r.width < 720;
    const count = Math.round(Math.min(5200, Math.max(1400, (r.width * r.height) / (mobile ? 140 : 210))));
    if (!dust || dust.count() !== count) {
      dust?.free();
      dust = new rust.Dust(count, w, h, (Math.random() * 2 ** 31) >>> 0);
    }
    dust.resize(w, h);
    dust.set_home(wordPoints(w, h, mobile));
    if (hud) hud.querySelector(".n")!.textContent = count.toLocaleString();
  };

  const draw = () => {
    const n = dust!.count();
    const pos = new Float32Array((rust.wasm_memory() as WebAssembly.Memory).buffer, dust!.positions(), n * 2);
    const s = 1.35 * dpr;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "rgba(237, 233, 227, 0.82)";
    for (let i = 0; i < n; i++) if (i % 11) ctx.fillRect(pos[i * 2]!, pos[i * 2 + 1]!, s, s);
    ctx.fillStyle = "#ff5a1f";
    for (let i = 0; i < n; i += 11) ctx.fillRect(pos[i * 2]!, pos[i * 2 + 1]!, s * 1.3, s * 1.3);
  };

  const frame = (now: number) => {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    if (!visible || document.hidden) return;
    const scatter = Math.min(1, Math.max(0, scrollY / (innerHeight * 0.9)));
    dust!.step(dt, pointer.x, pointer.y, scatter);
    draw();
    fps += (1 / Math.max(dt, 0.001) - fps) * 0.05;
    if (hud && now - lastHud > 500) {
      lastHud = now;
      hud.querySelector(".fps")!.textContent = String(Math.round(fps));
    }
  };

  setup();
  if (reduce) {
    // No motion: let the particles settle off-screen, then draw a single still frame.
    for (let i = 0; i < 240; i++) dust!.step(1 / 30, NaN, NaN, 0);
    draw();
  } else raf = requestAnimationFrame(frame);

  const move = (e: PointerEvent) => {
    const r = canvas.getBoundingClientRect();
    pointer = { x: (e.clientX - r.left) * dpr, y: (e.clientY - r.top) * dpr };
  };
  const leave = () => (pointer = { x: NaN, y: NaN });
  const io = new IntersectionObserver(([e]) => (visible = !!e?.isIntersecting));
  io.observe(canvas);
  let resizeTimer = 0;
  const resize = () => {
    clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      setup();
      if (reduce) draw();
    }, 150);
  };
  const section = canvas.parentElement!;
  section.addEventListener("pointermove", move, { passive: true });
  section.addEventListener("pointerleave", leave);
  addEventListener("resize", resize);
  return () => {
    cancelAnimationFrame(raf);
    io.disconnect();
    section.removeEventListener("pointermove", move);
    section.removeEventListener("pointerleave", leave);
    removeEventListener("resize", resize);
    dust?.free();
  };
}

export function Hero({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  const rust = useWasm(site);
  const canvas = useRef<HTMLCanvasElement>(null);
  const hud = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    if (!rust || !canvas.current) return;
    let stop: (() => void) | undefined;
    let cancelled = false;
    // Sample the word only once the display font is ready.
    document.fonts.ready.then(() => {
      if (!cancelled) stop = animate(canvas.current!, rust, hud.current);
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [rust]);

  return (
    <section className="hero" aria-labelledby="hero-title">
      <canvas ref={canvas} className={`dust${rust ? " live" : ""}`} aria-hidden="true" />
      <div className="hero-glow" aria-hidden="true" />
      <div className="hero-content">
        <p className="eyebrow hero-in" style={{ "--i": 0 } as React.CSSProperties}>
          <span className="pulse" aria-hidden="true" />
          {d.eyebrow}
        </p>
        <h1 id="hero-title" className="hero-title">
          {d.hero.map((line, i) => (
            <Split key={i} text={line} className="line" start={i * 3} reveal={false} />
          ))}
        </h1>
        <div className="hero-bottom">
          <p className="lead hero-in" style={{ "--i": 6 } as React.CSSProperties}>
            {d.lead}
          </p>
          <div className="hero-actions hero-in" style={{ "--i": 7 } as React.CSSProperties}>
            <a className="btn" href={`/${lang}/docs/quick-start/`} data-magnetic>
              {d.cta}
              <Arrow />
            </a>
            <a className="btn ghost" href={GITHUB} target="_blank" rel="noopener noreferrer" data-magnetic>
              {d.github}
            </a>
          </div>
        </div>
      </div>
      <p className="hud" ref={hud} aria-hidden="true">
        <span className="tag">Rust → wasm</span> {d.dust} · <span className="n">—</span> {d.particles} · <span className="fps">—</span> fps
      </p>
      <a className="scroll-hint" href="#manifesto" aria-label={d.scroll}>
        <span>{d.scroll}</span>
        <i />
      </a>
    </section>
  );
}
