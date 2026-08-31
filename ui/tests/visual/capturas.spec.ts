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

// Las dos pantallas de operar que quedaban.
for (const tema of ['dark'] as const) {
  test(`entorno · tema ${tema}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/')
    await page.getByRole('button', { name: 'pruebas', exact: true }).click()
    await page.getByRole('button', { name: /parada/ }).click()
    await page.getByRole('button', { name: 'entorno', exact: true }).click()
    await expect(page.locator('.claves li').first()).toBeVisible()
    // Con uno revelado, que es lo que hay que poder mirar: que se ve el reloj y
    // que los demás siguen ocultos.
    await page.getByLabel(/Revelar el valor de DB_PASSWORD/).click()
    await expect(page.locator('.valor')).toBeVisible()
    await page.screenshot({ path: `capturas/${tema}-entorno.png`, fullPage: true })
  })

  test(`monitor · tema ${tema}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/')
    await page.getByRole('button', { name: 'monitor', exact: true }).click()
    await expect(page.locator('.tabla tbody tr').first()).toBeVisible()
    await page.screenshot({ path: `capturas/${tema}-monitor.png`, fullPage: true })
  })
}

test('tráfico y métricas · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: 'pruebas', exact: true }).click()
  await page.getByRole('button', { name: /parada/ }).click()
  await page.getByRole('button', { name: 'tráfico', exact: true }).click()
  await expect(page.locator('.cifras').first()).toBeVisible()
  await page.screenshot({ path: 'capturas/dark-trafico.png', fullPage: true })
})

test('exec · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: 'pruebas', exact: true }).click()
  await page.getByRole('button', { name: /parada/ }).click()
  await page.getByRole('button', { name: 'exec', exact: true }).click()
  await page.locator('.entrada input').fill('rm -rf /srv/apps/parada/releases')
  await page.getByRole('button', { name: 'Ejecutar' }).click()
  await expect(page.locator('.peligro')).toBeVisible()
  await page.screenshot({ path: 'capturas/dark-exec.png', fullPage: true })
})

test('retirar y borrar · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: 'pruebas', exact: true }).click()
  await page.getByRole('button', { name: /parada/ }).click()
  await page.getByRole('button', { name: 'retirar', exact: true }).click()
  await page.getByText('Y borrar también sus datos').click()
  await page.getByRole('button', { name: 'Retirar y borrar los datos' }).click()
  await expect(page.locator('.hoja input')).toBeVisible()
  await page.screenshot({ path: 'capturas/dark-retirar.png', fullPage: true })
})

test('revertir · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: 'pruebas', exact: true }).click()
  await page.getByRole('button', { name: /parada/ }).click()
  await page.getByRole('button', { name: 'revertir', exact: true }).click()
  await expect(page.locator('.releases')).toBeVisible()
  await page.screenshot({ path: 'capturas/dark-revertir.png', fullPage: true })
})

test('alta de servidores · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: '+ servidores' }).click()
  await expect(page.locator('.lista li').first()).toBeVisible()
  // Se comprueba uno, para ver los dos estados a la vez: comprobado y sin
  // comprobar. Enumerar no visita, y eso tiene que verse.
  await page.locator('.acciones button').first().click()
  await expect(page.locator('.saludo').first()).toBeVisible()
  await page.screenshot({ path: 'capturas/dark-alta.png', fullPage: true })
})

test('asistente de web nueva · tema dark', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByRole('button', { name: 'vps-ovh', exact: true }).click()
  await page.getByRole('button', { name: 'nueva web' }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Nueva web')

  // Con el primer paso lleno, para que se vea el raíl con un tramo hecho y el
  // botón habilitado. Un formulario capturado vacío enseña el estado que menos
  // dice de él.
  await page.locator('input').first().fill('usuario/tienda')
  await expect(page.locator('.problema')).toHaveCount(0)
  await page.screenshot({ path: 'capturas/dark-nueva.png', fullPage: true })
})
