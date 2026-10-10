import { Layout } from "./components/Layout";
import type { Doc } from "./docs";
import { DocPage, DocsIndex } from "./pages/Docs";
import { Home } from "./pages/Home";
import { NotFound } from "./pages/NotFound";
import type { Route } from "./routes";
import "./styles/global.css";

export function App({ route, doc }: { route: Route; doc?: Doc }) {
  return (
    <Layout route={route}>
      {route.page === "home" && <Home lang={route.lang} />}
      {route.page === "docs" && <DocsIndex lang={route.lang} />}
      {route.page === "doc" && doc && <DocPage lang={route.lang} slug={route.slug} doc={doc} />}
      {route.page === "404" && <NotFound lang={route.lang} />}
    </Layout>
  );
}
