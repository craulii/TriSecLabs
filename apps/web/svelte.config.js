import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
export default {
  kit: {
    // adapter-static: genera un SPA bundle listo para ser servido por Axum.
    // fallback: 'index.html' activa el modo SPA — todas las rutas desconocidas
    // sirven index.html y SvelteKit maneja el routing en el cliente.
    // Esto también es lo correcto para Tauri (WebView no tiene servidor Node).
    adapter: adapter({
      pages:    'build',
      assets:   'build',
      fallback: 'index.html',
    }),

    alias: {
      // $lib ya viene configurado por SvelteKit, pero explicitarlo para IDEs
      $lib: './src/lib',
    },
  },
};
