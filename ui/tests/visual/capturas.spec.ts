import { expect, test } from '@playwright/test'

// Se capturan los dos temas y los cuatro escenarios. Los escenarios difíciles
// van primero porque son el grueso del producto: el estado feliz se diseña
// solo, y los demás no.
const ESCENARIOS = ['vps-ovh', 'pruebas', 'comprometido', 'recien-instalado'] as const

for (const tema of ['dark', 'light'] as const) {
  test.describe(`tema ${tema}`, () => {
    test.use({ colorScheme: tema })

    for (const escenario of ESCENARIOS) {
      test(`${escenario}`, async ({ page }) => {
        await page.goto('/')
        await page.getByRole('button', { name: escenario, exact: true }).click()
        await expect(page.getByRole('heading', { level: 1 })).toHaveText(escenario)
        await page.screenshot({ path: `capturas/${tema}-${escenario}.png`, fullPage: true })
      })
    }

    test('detalle de una app', async ({ page }) => {
      await page.goto('/')
      await page.getByRole('button', { name: 'pruebas', exact: true }).click()
      await page.getByRole('button', { name: /sin-vhost/ }).click()
      await page.screenshot({ path: `capturas/${tema}-detalle.png`, fullPage: true })
    })
  })
}
