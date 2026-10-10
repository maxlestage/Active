import { Helmet } from "react-helmet";
import { page_ } from "./helmet-tags";

export const Page = page_(Helmet);
export const Root = ({ i }: { i: number }) => <Page key={i} i={i} />;
