import { defineConfig } from '@playwright/test'

// Las capturas no son un adorno: son la única forma de comprobar que lo que
// dicen las pruebas del DOM se ve de verdad. Una prueba puede afirmar que el
// chip lleva la clase correcta y la interfaz seguir siendo ilegible.
export default defineConfig({
  testDir: './tests/visual',
  outputDir: './tests/visual/salida',
  use: {
    // El Chrome del sistema, no el de Playwright: sus binarios no cubren
    // Ubuntu 26.04 todavía. Y un Chromium de snap tampoco vale: su
    // confinamiento se rompe con «cannot find tracking cgroup» en cuanto se
    // lanzan varias instancias seguidas, que es exactamente lo que hace una
    // tanda de capturas. Se puede apuntar a otro con CHROMIUM.
    launchOptions: { executablePath: process.env.CHROMIUM || '/usr/bin/google-chrome' },
    baseURL: 'http://127.0.0.1:5174',
    colorScheme: 'dark',
    viewport: { width: 1280, height: 800 },
  },
  webServer: {
    // Se sirve el build y no `vite dev`, y no es un detalle: las capturas
    // tienen que mirar **lo que se publica**, no lo que sale de un servidor de
    // desarrollo con su HMR y sus módulos sin empaquetar. Además arranca en
    // milisegundos y no depende de que vite resuelva nada en caliente, que es
    // lo que se quedaba colgado en CI.
    command: 'npx vite preview --host 127.0.0.1 --port 5174 --strictPort',
    url: 'http://127.0.0.1:5174',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    // Sin esto, cuando el servidor no arranca lo único que se ve es «Timed out
    // waiting 60000ms», que no dice por qué.
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
