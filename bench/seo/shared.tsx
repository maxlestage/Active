// Same page for every library: 11 head tags + JSON-LD + a small body.
export const SITE = "Acme";
export const page = (i: number) => ({
  title: `Produit ${i}`,
  description: `Description du produit ${i}, livré en 24h partout en France.`,
  url: `https://acme.com/p/${i}`,
  image: `https://acme.com/img/${i}.png`,
  jsonLd: {
    "@context": "https://schema.org",
    "@type": "Product",
    name: `Produit ${i}`,
    offers: { "@type": "Offer", price: i, priceCurrency: "EUR" },
  },
});
export const expected = (i: number) => ({ ...page(i), title: `Produit ${i} · ${SITE}` });

export function Body({ i }: { i: number }) {
  return (
    <main>
      <h1>Produit {i}</h1>
      <ul>
        {Array.from({ length: 30 }, (_, k) => (
          <li key={k}>
            Option {k} — {i * k} €
          </li>
        ))}
      </ul>
    </main>
  );
}

export const template = (head: string, body: string) =>
  `<!DOCTYPE html><html><head><meta charset="utf-8">${head}</head><body><div id="root">${body}</div></body></html>`;
