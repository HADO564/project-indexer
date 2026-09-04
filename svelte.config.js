// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const dev = process.env.NODE_ENV === "development";

// Content security policy. The frontend loads nothing from the network — every
// font, icon and script ships in the bundle — so the policy can be tight.
//
// SvelteKit owns this rather than `tauri.conf.json` for one reason: the boot
// script in `index.html` is inline, and its hash changes on every build.
// `mode: "hash"` recomputes it automatically. Tauri sets a second, wider policy
// (see `app.security.csp`); a browser enforces the *intersection* of the two, so
// the strict half here is what actually binds.
//
// `style-src` needs `unsafe-inline` because a few components compute colours in
// a `style` attribute — `trackerColor(kind)` in the tracker badges most of all.
// Removing it means moving those to CSS custom properties first.
const csp = {
  mode: "hash",
  directives: {
    "default-src": ["self"],
    "script-src": ["self"],
    "style-src": ["self", "unsafe-inline"],
    "font-src": ["self"],
    "img-src": ["self", "data:"],
    // `ipc:` and `http://ipc.localhost` are how Tauri's `invoke` bridge reaches
    // the Rust side; the localhost entries are Vite's dev server and its HMR
    // socket, and are absent from a release build.
    "connect-src": dev
      ? ["self", "ipc:", "http://ipc.localhost", "ws://localhost:1420", "http://localhost:1420"]
      : ["self", "ipc:", "http://ipc.localhost"],
    "object-src": ["none"],
    "base-uri": ["self"],
    "form-action": ["none"],
  },
};

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: true,
  },
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    csp,
  },
};

export default config;
