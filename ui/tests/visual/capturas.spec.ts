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

// Las dos pantallas que el contrato terminado desbloqueó.
for (const tema of ['dark', 'light'] as const) {
  test.describe(`operar · tema ${tema}`, () => {
    test.use({ colorScheme: tema })

    test('diagnóstico', async ({ page }) => {
      await page.goto('/')
      await page.getByRole('button', { name: 'diagnóstico' }).click()
      await expect(page.locator('.lista li').first()).toBeVisible()
      await page.screenshot({ path: `capturas/${tema}-diagnostico.png`, fullPage: true })
    })

    test('log de una app', async ({ page }) => {
      await page.goto('/')
      await page.getByRole('button', { name: 'pruebas', exact: true }).click()
      await page.getByRole('button', { name: /parada/ }).click()
      await page.getByRole('button', { name: 'log', exact: true }).click()
      await expect(page.locator('.log')).toBeVisible()
      await page.screenshot({ path: `capturas/${tema}-log.png`, fullPage: true })
    })
  })
}
