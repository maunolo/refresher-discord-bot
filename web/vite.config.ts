import { fileURLToPath } from 'node:url';

import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';

// `npm run dev` serves the page with hot reload and sends /api, /auth and /demo to the
// Rust server: `cargo run` on 8080, or the demo with API_TARGET=http://127.0.0.1:8090.
const apiTarget = process.env.API_TARGET ?? 'http://127.0.0.1:8080';

/** In dev, `/overlay/{secret}/{overlay}` gets the overlay page, as the Rust server does. */
const overlayPages: Plugin = {
  name: 'overlay-pages',
  configureServer(server) {
    server.middlewares.use((req, _res, next) => {
      if (req.url?.startsWith('/overlay/')) req.url = '/overlay.html';
      next();
    });
  },
};

export default defineConfig({
  plugins: [react(), overlayPages],
  build: {
    rollupOptions: {
      // The admin page, and the OBS overlays as a page of their own.
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        overlay: fileURLToPath(new URL('./overlay.html', import.meta.url)),
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': apiTarget,
      '/auth': apiTarget,
      '/demo': apiTarget,
    },
  },
});
