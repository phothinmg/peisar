# Node.js usage: WASM

Use the WebAssembly build when the runtime cannot load native `.node`
bindings, or when you want to force the WASI path explicitly.

## Packages

The native package is:

```sh
npm i peisar
```

The WebAssembly package is:

```sh
npm i @peisar/peisar-wasm32-wasi
```

## How fallback works

The main `peisar` package prefers a native binary selected through npm
`optionalDependencies`. If no native binding can be loaded, the generated
loader can fall back to `@peisar/peisar-wasm32-wasi`.

Install both packages when you want automatic fallback:

```sh
npm i peisar @peisar/peisar-wasm32-wasi
```

## Environment variables

You can control binding selection with `NAPI_RS_FORCE_WASI`:

- `NAPI_RS_FORCE_WASI=true` — prefer the WASM build and keep the native
  binding as a lazy fallback
- `NAPI_RS_FORCE_WASI=error` — require the WASM build and fail when it is
  missing

## Browser-focused usage

For browser apps, import the WASM package directly:

```ts
import { Peisar } from "@peisar/peisar-wasm32-wasi";

const document = new Peisar("# Hello", { fragment: true });
console.log(document.html);
```

If you are using Vite, also follow the [Vite guide](./vite.md) for the
required isolation headers and optimizer configuration.
