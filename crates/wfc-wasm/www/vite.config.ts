import { defineConfig } from "vite";

export default defineConfig({
    // IPv4 only; the wasm glue resolves its .wasm relative to import.meta.url,
    // which Vite rewrites without extra plugins.
    server: { host: "127.0.0.1" },
});
