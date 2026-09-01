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

// El defecto que sólo se ve mirando: dos estados con el mismo color.
//
// «lo detecta Orbit» y «sin build» son dos respuestas distintas —la segunda le
// dice al servidor que esta app no se compila— y su distinción vive en
// estado.css, por token. Es exactamente la forma en que ya se rompió una vez:
// una regla de estado anidada pierde contra el <style> con ámbito de un
// componente, y el botón de borrar se quedó sin su rojo con el DOM en verde.
for (const tema of ['dark', 'light'] as const) {
  test(`un campo vaciado a propósito se distingue del que se detecta · ${tema}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/galeria.html')
    await page.waitForSelector('.anulacion--vacia')

    const vacia = page.locator('.anulacion--vacia').first()
    const detecta = page.locator('.estado-tri:not(.anulacion--vacia)').first()
    await detecta.waitFor({ state: 'visible' })

    const [cv, cd] = await Promise.all([
      vacia.evaluate((n) => getComputedStyle(n).color),
      detecta.evaluate((n) => getComputedStyle(n).color),
    ])
    expect(cv, 'si los dos estados son del mismo color, no hay dos estados').not.toBe(cd)

    // Y no sólo por color: el texto también los distingue, que es lo único que
    // sobrevive al daltonismo.
    expect(await vacia.textContent()).toContain('sin build')
    expect(await detecta.textContent()).toContain('lo detecta Orbit')
  })
}

// Los dos lados de una comparación tienen que verse distintos, y además
// llevarlo escrito. Misma comprobación que la del campo vaciado a propósito, y
// por el mismo motivo: una regla de estado anidada pierde contra el <style> con
// ámbito de un componente, y eso ya costó que un botón no saliera rojo.
for (const tema of ['dark', 'light'] as const) {
  test(`los dos lados de una comparación no se confunden · ${tema}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: tema })
    await page.goto('/galeria.html')
    await page.waitForSelector('.comparar .tabla')

    const a = page.locator('.comparar th.lado--a').first()
    const b = page.locator('.comparar th.lado--b').first()
    const [ca, cb] = await Promise.all([
      a.evaluate((n) => getComputedStyle(n).color),
      b.evaluate((n) => getComputedStyle(n).color),
    ])
    expect(ca, 'dos servidores del mismo color son un servidor').not.toBe(cb)

    // Y el color no es la señal: el alias va escrito en las dos cabeceras.
    expect((await a.textContent())?.trim()).toBe('produccion')
    expect((await b.textContent())?.trim()).toBe('staging')
  })
}
