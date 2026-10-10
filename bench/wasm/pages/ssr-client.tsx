import { hydrateRoot } from "react-dom/client";
import { App } from "./ssr-app";

hydrateRoot(document.getElementById("root")!, <App />);
