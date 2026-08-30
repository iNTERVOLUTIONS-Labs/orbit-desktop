import { expect, test } from '@playwright/test'

// La galería, en los dos temas. Es donde viven los estados difíciles, y los
// estados difíciles son el grueso del producto: el feliz se diseña solo.
for (const tema of ['dark', 'light'] as const) {
  test(`galería · tema ${tema}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/galeria.html')
    await page.waitForSelector('h1')
    // El esqueleto tarda 150 ms en aparecer, y es a propósito: así no parpadea
    // en las cargas que van rápidas. Sin esta espera la captura lo pillaba a
    // opacidad cero y el recuadro salía vacío — que parecía un defecto de
    // contraste y era el retardo haciendo su trabajo.
    await page.waitForTimeout(400)
    await page.screenshot({ path: `capturas/${tema}-galeria.png`, fullPage: true })
  })

  test(`el esqueleto se ve · tema ${tema}`, async ({ page }) => {
    // Regresión de un defecto que sólo se vio mirando una captura: el recuadro
    // de carga salía **vacío**. La causa no era el contraste sino los 150 ms de
    // retardo, que están a propósito para que no parpadee en las cargas
    // rápidas. Aquí se comprueban las dos cosas: que aparece, y que se
    // distingue del panel que hay debajo.
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/galeria.html')
    const fila = page.locator('.esqueleto .fila').first()
    await fila.waitFor({ state: 'visible' })
    await page.waitForTimeout(400)

    const opacidad = await fila.evaluate((n) =>
      Number(getComputedStyle(n.parentElement!).opacity),
    )
    expect(opacidad, 'el esqueleto tiene que haber aparecido').toBeGreaterThan(0.9)

    const [fondoFila, fondoPanel] = await fila.evaluate((n) => [
      getComputedStyle(n).backgroundColor,
      getComputedStyle(n.parentElement!.parentElement!).backgroundColor,
    ])
    expect(fondoFila, 'una fila del mismo color que el panel es una caja vacía')
      .not.toBe(fondoPanel)
  })
}
