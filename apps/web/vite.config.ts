import { sveltekit }   from '@sveltejs/kit/vite';
import tailwindcss     from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],

  server: {
    port: 5173,
    // En desarrollo, proxiar /api al servidor Axum local.
    // El navegador ve todo como mismo origen (5173) → sin CORS.
    proxy: {
      '/api': {
        target:       'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },

  // Para Tauri: el proceso Vite debe escuchar en localhost, no en 0.0.0.0
  // cuando se usa `tauri dev`. Tauri sobreescribe esto internamente.
});
