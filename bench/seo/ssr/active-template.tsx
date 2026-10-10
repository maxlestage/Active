// Template mode (Vite SSR style): React renders only the app, head is injected.
import { renderWithHead } from "active-react/server";
import { Root } from "../libs/active";
import { template } from "../shared";

export const ssr = (i: number) => {
  const { head, html } = renderWithHead(<Root i={i} />);
  return template(head, html);
};
