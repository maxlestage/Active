// Baseline: hand-written React 19 tags, no library.
import { Body, SITE, page } from "../shared";

function Page({ i }: { i: number }) {
  const p = page(i);
  return (
    <>
      <title>{`${p.title} · ${SITE}`}</title>
      <meta name="description" content={p.description} />
      <link rel="canonical" href={p.url} />
      <meta property="og:title" content={p.title} />
      <meta property="og:description" content={p.description} />
      <meta property="og:type" content="website" />
      <meta property="og:url" content={p.url} />
      <meta property="og:image" content={p.image} />
      <meta property="og:site_name" content={SITE} />
      <meta name="twitter:card" content="summary_large_image" />
      <meta name="twitter:site" content="@acme" />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(p.jsonLd) }} />
      <Body i={i} />
    </>
  );
}

export const Root = ({ i }: { i: number }) => <Page key={i} i={i} />;
