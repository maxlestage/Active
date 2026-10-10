import { useHead } from "@unhead/react";
import { Body, SITE, page } from "../shared";

export function Page({ i }: { i: number }) {
  const p = page(i);
  useHead({
    title: p.title,
    titleTemplate: `%s · ${SITE}`,
    meta: [
      { name: "description", content: p.description },
      { property: "og:title", content: p.title },
      { property: "og:description", content: p.description },
      { property: "og:type", content: "website" },
      { property: "og:url", content: p.url },
      { property: "og:image", content: p.image },
      { property: "og:site_name", content: SITE },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:site", content: "@acme" },
    ],
    link: [{ rel: "canonical", href: p.url }],
    script: [{ type: "application/ld+json", innerHTML: JSON.stringify(p.jsonLd) }],
  });
  return <Body i={i} />;
}
