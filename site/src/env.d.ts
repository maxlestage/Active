/// <reference types="vite/client" />

declare module "virtual:docs-manifest" {
  const manifest: Record<string, Record<string, { title: string; description: string }>>;
  export default manifest;
}

declare module "*.md" {
  const doc: import("./docs").Doc & { blocks: string[] };
  export default doc;
}

interface Window {
  __SITE__: string;
}
