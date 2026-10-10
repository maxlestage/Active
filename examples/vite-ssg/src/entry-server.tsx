import { renderWithHead } from "active-react/server";
import { App, routes } from "./App";

export const urls = Object.keys(routes);
export const render = (url: string) => renderWithHead(<App url={url} />);
