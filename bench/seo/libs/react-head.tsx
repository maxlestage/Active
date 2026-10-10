import { HeadProvider, Link, Meta, Title } from "react-head";
import { Body, SITE, page } from "../shared";

export function Page({ i }: { i: number }) {
  const p = page(i);
  return (
    <>
      <Title>{`${p.title} · ${SITE}`}</Title>
      <Meta name="description" content={p.description} />
      <Link rel="canonical" href={p.url} />
      <Meta property="og:title" content={p.title} />
      <Meta property="og:description" content={p.description} />
      <Meta property="og:type" content="website" />
      <Meta property="og:url" content={p.url} />
      <Meta property="og:image" content={p.image} />
      <Meta property="og:site_name" content={SITE} />
      <Meta name="twitter:card" content="summary_large_image" />
      <Meta name="twitter:site" content="@acme" />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(p.jsonLd) }} />
      <Body i={i} />
    </>
  );
}

export const Root = ({ i }: { i: number }) => (
  <HeadProvider>
    <Page key={i} i={i} />
  </HeadProvider>
);
