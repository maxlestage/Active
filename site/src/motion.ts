// All page motion in one small vanilla module (no animation library):
// scroll reveals, counters, magnetic buttons, spotlight cards, custom cursor,
// header state, scroll progress and copy buttons. Returns a cleanup function.
const fine = () => matchMedia("(pointer: fine)").matches;
const reduced = () => document.documentElement.classList.contains("reduce");

export function initMotion(lang: string): () => void {
  const root = document.documentElement;
  const off: Array<() => void> = [];
  const on = <K extends keyof WindowEventMap>(target: Window | Document | HTMLElement, type: K | string, fn: (e: any) => void, opts?: AddEventListenerOptions) => {
    target.addEventListener(type, fn, opts);
    off.push(() => target.removeEventListener(type, fn, opts));
  };
  root.classList.add("hydrated");

  // Reveals: elements get `.is-in` the first time they enter the viewport.
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.classList.add("is-in");
        io.unobserve(e.target);
        if (e.target instanceof HTMLElement && e.target.dataset.count !== undefined) count(e.target, lang);
      }
    },
    { rootMargin: "0px 0px -8% 0px", threshold: 0.12 },
  );
  document.querySelectorAll("[data-reveal], [data-count]").forEach((el) => io.observe(el));
  off.push(() => io.disconnect());

  const scrubs = reduced() ? [] : [...document.querySelectorAll<HTMLElement>("[data-scrub]")];
  scrubs.forEach((el) => el.classList.add("scrubbing"));

  // Header: solid after the hero, hidden while scrolling down. Progress bar: --progress.
  let last = scrollY;
  let ticking = false;
  const onScroll = () => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      ticking = false;
      const y = scrollY;
      root.classList.toggle("scrolled", y > 40);
      root.classList.toggle("hide-header", y > last && y > 400);
      last = y;
      const max = document.documentElement.scrollHeight - innerHeight;
      root.style.setProperty("--progress", String(max > 0 ? y / max : 0));
      root.style.setProperty("--scroll", String(y));
      for (const el of scrubs) {
        const r = el.getBoundingClientRect();
        const p = (innerHeight * 0.85 - r.top) / (r.height + innerHeight * 0.45);
        el.style.setProperty("--p", String(Math.min(1, Math.max(0, p))));
      }
    });
  };
  on(window, "scroll", onScroll, { passive: true });
  onScroll();

  // Copy buttons of code blocks (docs and home).
  on(document, "click", (e: MouseEvent) => {
    const btn = (e.target as HTMLElement).closest<HTMLButtonElement>(".copy");
    if (!btn) return;
    const code = btn.parentElement?.querySelector("pre")?.textContent ?? "";
    navigator.clipboard?.writeText(code).then(() => {
      btn.classList.add("done");
      setTimeout(() => btn.classList.remove("done"), 1400);
    });
  });

  if (fine() && !reduced()) off.push(cursor(), magnetic(), spotlight());
  return () => off.forEach((f) => f());
}

function count(el: HTMLElement, lang: string) {
  const target = Number(el.dataset.count);
  const decimals = Number(el.dataset.decimals ?? 0);
  const fmt = new Intl.NumberFormat(lang, { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
  if (reduced()) return void (el.textContent = fmt.format(target));
  const start = performance.now();
  const tick = (now: number) => {
    const p = Math.min(1, (now - start) / 1400);
    el.textContent = fmt.format(target * (1 - Math.pow(1 - p, 4)));
    if (p < 1) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

function cursor() {
  const el = document.querySelector<HTMLElement>(".cursor");
  if (!el) return () => {};
  let x = innerWidth / 2, y = innerHeight / 2, cx = x, cy = y, raf = 0;
  const move = (e: PointerEvent) => {
    x = e.clientX;
    y = e.clientY;
    el.classList.add("on");
    const target = (e.target as HTMLElement).closest("a, button, input, textarea, [data-cursor]");
    el.classList.toggle("hover", !!target);
  };
  const leave = () => el.classList.remove("on");
  const down = () => el.classList.add("down");
  const up = () => el.classList.remove("down");
  const loop = () => {
    cx += (x - cx) * 0.2;
    cy += (y - cy) * 0.2;
    el.style.setProperty("--x", `${x}px`);
    el.style.setProperty("--y", `${y}px`);
    el.style.setProperty("--cx", `${cx}px`);
    el.style.setProperty("--cy", `${cy}px`);
    raf = requestAnimationFrame(loop);
  };
  raf = requestAnimationFrame(loop);
  addEventListener("pointermove", move, { passive: true });
  document.addEventListener("pointerleave", leave);
  addEventListener("pointerdown", down);
  addEventListener("pointerup", up);
  return () => {
    cancelAnimationFrame(raf);
    removeEventListener("pointermove", move);
    document.removeEventListener("pointerleave", leave);
    removeEventListener("pointerdown", down);
    removeEventListener("pointerup", up);
  };
}

function magnetic() {
  const els = [...document.querySelectorAll<HTMLElement>("[data-magnetic]")];
  const handlers = els.map((el) => {
    const move = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      const dx = (e.clientX - (r.left + r.width / 2)) / r.width;
      const dy = (e.clientY - (r.top + r.height / 2)) / r.height;
      el.style.transform = `translate(${dx * 12}px, ${dy * 10}px)`;
    };
    const leave = () => (el.style.transform = "");
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerleave", leave);
    return () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerleave", leave);
    };
  });
  return () => handlers.forEach((h) => h());
}

function spotlight() {
  const move = (e: PointerEvent) => {
    const card = (e.target as HTMLElement).closest<HTMLElement>("[data-spot]");
    if (!card) return;
    const r = card.getBoundingClientRect();
    card.style.setProperty("--mx", `${e.clientX - r.left}px`);
    card.style.setProperty("--my", `${e.clientY - r.top}px`);
  };
  addEventListener("pointermove", move, { passive: true });
  return () => removeEventListener("pointermove", move);
}
