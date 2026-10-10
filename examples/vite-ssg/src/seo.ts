import { createSeo } from "active-react";

// Site-wide defaults, written once.
export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  image: "/og.png",
  locale: "fr_FR",
});
