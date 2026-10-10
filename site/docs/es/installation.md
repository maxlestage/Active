---
title: Instalación
description: Instala active-react desde GitHub hoy o desde npm cuando se publique. Incluye los tipos de TypeScript y las apps solo SEO nunca envían el código WASM.
---

active-react es un único paquete con dos puntos de entrada. Instalarlo requiere un solo comando; la toolchain de Rust solo hace falta si usas la parte de WebAssembly.

## Requisitos

| Herramienta | Versión | Necesaria para |
| --- | --- | --- |
| `react` y `react-dom` | 19 o posterior | Todo (peer dependencies) |
| Node.js | 20 o posterior | Renderizado en servidor y generación estática |
| Rust | Estable, con el target `wasm32-unknown-unknown` | Solo la parte de WebAssembly |
| TypeScript | Opcional | Los tipos vienen incluidos |

¿Por qué React 19? Desde la versión 19, React traslada por sí mismo `<title>`, `<meta>` y `<link>` al `<head>`. active-react se basa en esa funcionalidad, por lo que React 18 no está soportado.

Comprueba tus versiones:

```sh
npm ls react react-dom
```

Si todavía usas React 18, actualiza primero:

```sh
npm i react@latest react-dom@latest
npm i -D @types/react@latest @types/react-dom@latest
```

## Instalar

### Desde GitHub (hoy)

El paquete todavía no está en npm. Instálalo directamente desde el repositorio:

```sh
npm i github:maxlestage/Active
```

npm clona el repositorio y ejecuta su script `prepare`, que compila la biblioteca en `dist/`. El paquete se sigue llamando `active-react`, así que tus imports son los mismos que con la futura versión de npm. Tu `package.json` recibe esta línea:

```json
{
  "dependencies": {
    "active-react": "github:maxlestage/Active"
  }
}
```

> **Consejo:** Añade `#` y el hash de un commit (o una etiqueta de Git) para fijar una versión exacta: `npm i github:maxlestage/Active#<commit-hash>`.

### Desde npm (cuando se publique)

```sh
npm i active-react
```

Lo mismo con otros gestores de paquetes:

```sh
pnpm add active-react
yarn add active-react
bun add active-react
```

> **Nota:** pnpm, Yarn y Bun también aceptan `github:maxlestage/Active`. Algunos de ellos bloquean por defecto los scripts de instalación de las dependencias; si `node_modules/active-react/dist` no existe después de la instalación, autoriza el script de build del paquete o usa npm.

## Puntos de entrada

| Importar desde | Exporta | Úsalo en |
| --- | --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` | Tus componentes, renderizados en el servidor o en el navegador |
| `active-react/server` | `renderWithHead` | Solo código de servidor |

```tsx
import { Seo, createSeo, wasm, useWasm, type SeoProps, type Wasm } from "active-react";
import { renderWithHead } from "active-react/server";
```

> **Advertencia:** Nunca importes `active-react/server` desde código del navegador. Arrastra `react-dom/server`, que no quieres en tu bundle del cliente.

## TypeScript

Los tipos vienen con el paquete: no hay ningún `@types/active-react` que instalar. Pasa el cursor sobre cualquier prop en tu editor para leer su documentación.

El paquete usa el campo `exports` de `package.json`, así que TypeScript necesita una resolución de módulos moderna para encontrar `active-react/server`:

```json
{
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
```

`"bundler"` es el valor por defecto en las plantillas de Vite. `"node16"` y `"nodenext"` también funcionan. El antiguo ajuste `"node"` no puede resolver el punto de entrada `/server`.

El paquete es solo ESM (`"type": "module"`): usa `import`, no `require`.

> **Nota:** Importar un archivo `.wasm` con el sufijo `?url` de Vite requiere los tipos de cliente de Vite. Las plantillas de Vite ya los incluyen (`vite/client`).

## Tree-shaking

El paquete declara `"sideEffects": false`, así que tu bundler conserva solo lo que importas:

| Importas | Se añade a tu bundle del cliente |
| --- | ---: |
| `Seo` y/o `createSeo` | 0,74 kB gzip |
| `wasm` y `useWasm` | 0,60 kB gzip |
| Todo | unos 1,3 kB gzip |

Una app que solo usa SEO nunca envía el cargador de WebAssembly, y una app que solo usa Rust nunca envía el código SEO. Estos tamaños provienen de los [benchmarks](../benchmarks/): minificados con esbuild, sin contar React ni React DOM, ya que tu app ya los incluye.

## Próximos pasos

- Sigue el [inicio rápido](../quick-start/) para crear una pequeña app con SEO y Rust.
- Lee sobre [el componente `<Seo>`](../seo-component/).
- Configura la [toolchain de Rust](../rust-setup/) si piensas usar WebAssembly.
