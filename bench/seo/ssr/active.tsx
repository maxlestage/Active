import { renderToString } from "react-dom/server";
import { Root } from "../libs/active";

export const ssr = (i: number) =>
  "<!DOCTYPE html>" +
  renderToString(
    <html>
      <head>
        <meta charSet="utf-8" />
      </head>
      <body>
        <div id="root">
          <Root i={i} />
        </div>
      </body>
    </html>,
  );
