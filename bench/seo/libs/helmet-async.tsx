import { Helmet, HelmetProvider } from "react-helmet-async";
import { page_ } from "./helmet-tags";

export const Page = page_(Helmet);
export const Root = ({ i }: { i: number }) => (
  <HelmetProvider>
    <Page key={i} i={i} />
  </HelmetProvider>
);
