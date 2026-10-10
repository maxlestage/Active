import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "@fontsource/instrument-serif/400-italic.css";
import { createRoot, hydrateRoot } from "react-dom/client";
import { App } from "./App";
import { SLUGS, loadDoc } from "./docs";
import { parse } from "./routes";

const route = parse(location.pathname, SLUGS);
// Load the current docs page first so the first client render matches the prerendered HTML.
const doc = route.page === "doc" ? await loadDoc(route.lang, route.slug) : undefined;
const root = document.getElementById("root")!;
const app = <App route={route} doc={doc} />;
if (root.firstElementChild) hydrateRoot(root, app);
else createRoot(root).render(app);
