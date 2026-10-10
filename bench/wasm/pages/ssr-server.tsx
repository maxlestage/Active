import { renderWithHead } from "active-react/server";
import { App } from "./ssr-app";

const { head, html } = renderWithHead(<App />);
process.stdout.write(JSON.stringify({ head, html }));
