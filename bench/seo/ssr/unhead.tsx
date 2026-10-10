import { renderToString } from "react-dom/server";
import { UnheadProvider, createHead, renderSSRHead } from "@unhead/react/server";
import { Page } from "../libs/unhead-page";
import { template } from "../shared";

export const ssr = (i: number) => {
  const head = createHead();
  const body = renderToString(
    <UnheadProvider value={head}>
      <Page i={i} />
    </UnheadProvider>,
  );
  return template(renderSSRHead(head).headTags, body);
};
