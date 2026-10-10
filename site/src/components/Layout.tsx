import { useEffect, type ReactNode } from "react";
import { initMotion } from "../motion";
import type { Route } from "../routes";
import { Footer } from "./Footer";
import { Header } from "./Header";

export function Layout({ route, children }: { route: Route; children: ReactNode }) {
  useEffect(() => initMotion(route.lang), [route.lang]);
  return (
    <>
      <div className="progress" aria-hidden="true" />
      <Header route={route} />
      <main id="main" className={`page page-${route.page}`}>
        {children}
      </main>
      <Footer route={route} />
      <div className="cursor" aria-hidden="true">
        <span className="dot" />
        <span className="ring" />
      </div>
      <div className="grain" aria-hidden="true" />
    </>
  );
}
