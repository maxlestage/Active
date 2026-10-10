import { useWasm } from "active-react";
import { Seo } from "./seo";
import { hello } from "./wasm";

function Home() {
  const rust = useWasm(hello); // undefined while loading (and during SSR)
  return (
    <main>
      <Seo
        title="Accueil"
        description="Exemple active-react : SEO + Rust/WASM avec React 19."
        canonical="/"
        jsonLd={{ "@context": "https://schema.org", "@type": "WebSite", name: "Acme", url: "https://acme.com" }}
      />
      <h1>active-react</h1>
      <p id="greet">{rust ? rust.greet("toi") : "Chargement du module Rust…"}</p>
      <p id="primes">{rust && `${rust.count_primes(1_000_000)} nombres premiers sous 1 000 000`}</p>
      <a href="/about/">À propos</a>
    </main>
  );
}

function About() {
  return (
    <main>
      <Seo title="À propos" description="Pourquoi active-react est si léger." canonical="/about/" />
      <h1>À propos</h1>
      <a href="/">Accueil</a>
    </main>
  );
}

export const routes: Record<string, () => React.ReactNode> = { "/": Home, "/about/": About };

export function App({ url }: { url: string }) {
  const Page = routes[url] ?? Home;
  return <Page />;
}
