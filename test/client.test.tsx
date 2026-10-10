// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Seo, createSeo } from "../src/index.js";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
const render = (node: React.ReactNode) =>
  act(() => {
    root ??= createRoot(document.body.appendChild(document.createElement("div")));
    root.render(node);
  });
const meta = (sel: string) => document.head.querySelectorAll(`meta[${sel}]`);

afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  document.head.innerHTML = "";
  document.body.innerHTML = "";
});

describe("Seo (client)", () => {
  it("writes and updates <head> on navigation", async () => {
    const SiteSeo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
    const Page = ({ id }: { id: string }) => <SiteSeo title={id} description={`about ${id}`} canonical={`/${id}`} />;
    const app = (id: string) => <Page key={id} id={id} />;

    await render(app("a"));
    expect(document.title).toBe("a · Acme");
    expect(document.head.querySelector('link[rel="canonical"]')?.getAttribute("href")).toBe("https://acme.com/a");

    await render(app("b"));
    expect(document.title).toBe("b · Acme");
    expect(document.head.querySelectorAll("title")).toHaveLength(1);
    expect(meta('name="description"')).toHaveLength(1);
    expect(meta('name="description"')[0]!.getAttribute("content")).toBe("about b");
    expect(document.head.querySelectorAll('link[rel="canonical"]')).toHaveLength(1);
  });

  it("removes its tags on unmount", async () => {
    await render(<Seo title="x" description="y" />);
    expect(document.head.children.length).toBeGreaterThan(0);
    await render(null);
    expect(document.head.children).toHaveLength(0);
  });

  it("takes over the static <title> of index.html", async () => {
    document.head.innerHTML = "<title>Vite App</title>";
    await render(<Seo title="Real title" />);
    expect(document.title).toBe("Real title");
  });
});
