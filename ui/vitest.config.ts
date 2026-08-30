import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// La configuración de las pruebas va aparte de la de la aplicación por una
// razón concreta: aquí hace falta `resolve.conditions: ['browser']` en el
// NIVEL RAÍZ, no dentro de `test`. Sin eso, Svelte 5 resuelve a su build de
// servidor dentro de vitest y `mount` no existe — y el síntoma, «mount is not
// available on the server», no se parece en nada a la causa, que es la
// resolución de condiciones del paquete.
export default defineConfig({
  plugins: [svelte({ hot: false })],
  resolve: { conditions: ['browser'] },
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/preparar.ts'],
    globals: true,
  },
})
