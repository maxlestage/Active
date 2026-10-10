import { createSeo } from "active-react";

export const GITHUB = "https://github.com/maxlestage/Active";
export const HEROKU_DEPLOY = `https://heroku.com/deploy?template=${GITHUB}`;

// On the server the origin is a placeholder that server.js (or SITE_URL at build time) replaces,
// so canonical and Open Graph URLs are absolute on any domain. The client reads the same value.
const injected = typeof window === "undefined" ? "__ORIGIN__" : window.__SITE__;
export const ORIGIN = injected.startsWith("__") && typeof window !== "undefined" ? window.location.origin : injected;

export const Seo = createSeo({
  siteName: "active",
  titleTemplate: "%s — active",
  baseUrl: ORIGIN,
  image: "/og.png",
});
