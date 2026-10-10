import type { CSSProperties, ElementType } from "react";

/**
 * Splits text into masked words for staggered reveals. `*text*` becomes an italic accent.
 * Word spans are rendered on the server too, so the animation needs no layout work at runtime.
 */
export function Split({
  text,
  as: Tag = "span",
  className = "",
  start = 0,
  reveal = true,
}: {
  text: string;
  as?: ElementType;
  className?: string;
  start?: number;
  /** Reveal when scrolled into view (false: animated by CSS on load). */
  reveal?: boolean;
}) {
  let i = start;
  const parts = text.split(/(\*[^*]+\*)/).filter(Boolean);
  return (
    <Tag className={`split ${className}`} data-reveal={reveal ? "" : undefined}>
      {parts.map((part, p) => {
        const accent = part.startsWith("*");
        const words = (accent ? part.slice(1, -1) : part).split(/(\s+)/);
        const nodes = words.map((w, k) =>
          /^\s+$/.test(w) || !w ? (
            w
          ) : (
            <span className="w" key={k}>
              <span style={{ "--i": i++ } as CSSProperties}>{w}</span>
            </span>
          ),
        );
        return accent ? <em key={p}>{nodes}</em> : <span key={p}>{nodes}</span>;
      })}
    </Tag>
  );
}

/** Plain text without the `*accent*` markers (for meta tags and aria labels). */
export const plain = (text: string) => text.replace(/\*/g, "");
