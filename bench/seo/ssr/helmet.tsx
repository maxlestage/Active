import { renderToString } from "react-dom/server";
import { Helmet } from "react-helmet";
import { Root } from "../libs/helmet";
import { template } from "../shared";

export const ssr = (i: number) => {
  const body = renderToString(<Root i={i} />);
  const h = Helmet.renderStatic();
  return template(h.title.toString() + h.meta.toString() + h.link.toString() + h.script.toString(), body);
};
