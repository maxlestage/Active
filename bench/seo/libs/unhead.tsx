import { UnheadProvider, createHead } from "@unhead/react/client";
import { Page } from "./unhead-page";

const head = createHead();
export const Root = ({ i }: { i: number }) => (
  <UnheadProvider value={head}>
    <Page key={i} i={i} />
  </UnheadProvider>
);
