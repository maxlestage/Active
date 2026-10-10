import type { ReactElement } from "react";
import { renderToString } from "react-dom/server";
import { HeadProvider } from "react-head";
import { Page } from "../libs/react-head";
import { template } from "../shared";

export const ssr = (i: number) => {
  const tags: ReactElement[] = [];
  const body = renderToString(
    <HeadProvider headTags={tags}>
      <Page i={i} />
    </HeadProvider>,
  );
  return template(renderToString(tags), body);
};
