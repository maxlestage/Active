import type { ReactNode } from "react";
import { renderToString } from "react-dom/server";

const ID = "__active_head__";
const MARKER = `<template id="${ID}"></template>`;

/**
 * For HTML templates (e.g. Vite SSR `<!--app-head-->` / `<!--app-html-->`) where React
 * does not render `<html>` itself: returns the hoisted head tags and the body separately.
 * If your React tree renders `<html>`, use `renderToString` / streaming directly instead.
 */
export function renderWithHead(node: ReactNode): { head: string; html: string } {
  // React emits hoisted tags (title, meta, link…) before the first regular element.
  const out = renderToString(
    <>
      <template id={ID} />
      {node}
    </>,
  );
  const i = out.indexOf(MARKER);
  return { head: out.slice(0, i), html: out.slice(i + MARKER.length) };
}
