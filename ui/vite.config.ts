import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  server: { port: 5174, strictPort: true },
  build: {
    // La ventana corre en el WebView del sistema, que en las tres plataformas
    // es reciente: no hace falta transpilar a lo que entendía un navegador de
    // hace cinco años, y el paquete sale más pequeño.
    target: 'es2022',
    rollupOptions: {
      // La galería es una entrada aparte que sirve para mirar y capturar el
      // sistema de diseño. La primera versión la metía SIEMPRE en el build, y
      // Tauri empaqueta todo `ui/dist`: el comentario decía que no entraba en el
      // envoltorio y entraba. Ahora sólo con `vite build --mode galeria`; en
      // desarrollo `vite` la sirve igual, porque ahí no hay build.
      input: mode === 'galeria' ? { index: 'index.html', galeria: 'galeria.html' } : { index: 'index.html' },
    },
  },
}))
