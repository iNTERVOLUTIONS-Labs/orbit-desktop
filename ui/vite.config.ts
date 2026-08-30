import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  server: { port: 5174, strictPort: true },
  build: {
    rollupOptions: {
      // La galería es una entrada aparte y NO entra en el envoltorio de
      // escritorio: Tauri sirve `index.html`. Que exista en el build de
      // desarrollo es lo que permite capturarla.
      input: { index: 'index.html', galeria: 'galeria.html' },
    },
  },
})
