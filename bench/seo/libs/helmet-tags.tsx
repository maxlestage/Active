// Shared by react-helmet and react-helmet-async (same <Helmet> API).
import { Body, SITE, page } from "../shared";

export function page_(Helmet: any) {
  return function Page({ i }: { i: number }) {
    const p = page(i);
    return (
      <>
        <Helmet titleTemplate={`%s · ${SITE}`}>
          <title>{p.title}</title>
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
          <script type="application/ld+json">{JSON.stringify(p.jsonLd)}</script>
        </Helmet>
        <Body i={i} />
      </>
    );
  };
}
