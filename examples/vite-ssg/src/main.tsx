import { createRoot, hydrateRoot } from "react-dom/client";
import { App } from "./App";

const root = document.getElementById("root")!;
const app = <App url={location.pathname} />;
// Prerendered pages are hydrated; `npm run dev` renders from scratch.
if (root.firstElementChild) hydrateRoot(root, app);
else createRoot(root).render(app);
