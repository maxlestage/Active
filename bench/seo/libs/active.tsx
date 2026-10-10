import { createSeo } from "active-react";
import { Body, SITE, page } from "../shared";

const Seo = createSeo({ siteName: SITE, titleTemplate: `%s · ${SITE}`, twitter: "@acme" });

function Page({ i }: { i: number }) {
  const p = page(i);
  return (
    <>
      <Seo title={p.title} description={p.description} canonical={p.url} image={p.image} jsonLd={p.jsonLd} />
      <Body i={i} />
    </>
  );
}

export const Root = ({ i }: { i: number }) => <Page key={i} i={i} />;
