import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), sveltekit()],

  test: {
    // Tests live in `src/tests/`, mirroring the tree they cover, the same way
    // `crates/core/src/tests/` mirrors the Rust modules — so a source file
    // reads as source. The glob deliberately stays broad rather than pinned to
    // `src/tests/**`: a test written next to its source is in the wrong place,
    // but it should still *run* and be moved, not sit there silently skipped.
    include: ["src/**/*.test.ts"],
    environment: "node",
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching the Rust side. `target/` sits at the
      //    workspace root (not under `src-tauri/`), and watching it races the
      //    linker writing the exe — an EBUSY crash of the dev watcher.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
}));
