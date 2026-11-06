import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Two entry pages: the tiny splash (no framework, paints first) and the main app.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: 'chrome110', // WebView2 is evergreen
    rollupOptions: {
      input: {
        main: 'index.html',
        splash: 'splash.html',
      },
    },
  },
});
