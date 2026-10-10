import { LANGS, translate, type Route } from "../routes";

/** hreflang links for every language of a route, plus x-default (English). */
export function alternates(route: Route): Record<string, string> {
  const out: Record<string, string> = {};
  for (const l of LANGS) out[l] = translate(route, l);
  out["x-default"] = translate(route, "en");
  return out;
}
