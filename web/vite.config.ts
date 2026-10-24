import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// `npm run dev` serves the page with hot reload and sends /api, /auth and /demo to the
// Rust server: `cargo run` on 8080, or the demo with API_TARGET=http://127.0.0.1:8090.
const apiTarget = process.env.API_TARGET ?? 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': apiTarget,
      '/auth': apiTarget,
      '/demo': apiTarget,
    },
  },
});
