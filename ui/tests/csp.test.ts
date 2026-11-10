/**
 * La CSP del `orbit.json`, Orbit 1.5.1.
 *
 * Una política que Orbit rechaza **aborta el despliegue** con la versión
 * anterior en producción, porque acaba entre comillas dobles dentro de un vhost
 * que se enlaza antes del `nginx -t`. Así que el generador tiene que rechazar
 * exactamente lo mismo que `_csp_check`, con el mismo motivo, y nunca emitirla
 * «arreglada».
 */
import { describe, expect, it } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'

import OrbitJson from '../src/componentes/OrbitJson.svelte'
import { generar, problemaDeLaCsp } from '../src/lib/orbitjson'
import { sabe, type AppInfo, type Estado } from '../src/lib/contrato'

const ESTADO: Estado = {
  service: null, port: null, ssl: true, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 1, last_deploy: '20261109-120000', last_deploy_sha: 'abc',
}

const info = (config: Record<string, string>): AppInfo => ({
  name: 'web',
  path: '/srv/apps/web',
  config: { type: 'static', outdir: 'dist', ...config },
  state: ESTADO,
  releases: ['20261109-120000'],
})

const leer = (g: { texto: string }) => JSON.parse(g.texto)

describe('las reglas de _csp_check, una por una', () => {
  it('una política normal vale', () => {
    expect(problemaDeLaCsp("default-src 'self'; img-src 'self' data:")).toBeNull()
  })

  it('vacía o sólo blancos es no haber puesto nada, y vale', () => {
    expect(problemaDeLaCsp('')).toBeNull()
    expect(problemaDeLaCsp('   \t ')).toBeNull()
  })

  it('rechaza lo que rompería el vhost', () => {
    expect(problemaDeLaCsp('default-src "self"')).toMatch(/comillas dobles/)
    expect(problemaDeLaCsp('default-src a\\b')).toMatch(/barras invertidas/)
    expect(problemaDeLaCsp('default-src $host')).toMatch(/«\$»/)
    expect(problemaDeLaCsp('default-src {x}')).toMatch(/«\{»/)
    expect(problemaDeLaCsp('default-src a\nb')).toMatch(/saltos de línea/)
    expect(problemaDeLaCsp('default-src a\x01')).toMatch(/control/)
    expect(problemaDeLaCsp('x'.repeat(4097))).toMatch(/4096/)
  })

  it('un «;» final no vale, pero los blancos de detrás no cuentan', () => {
    expect(problemaDeLaCsp("default-src 'self';")).toMatch(/«;»/)
    expect(problemaDeLaCsp("default-src 'self';   ")).toMatch(/«;»/)
    expect(problemaDeLaCsp("  default-src 'self'  ")).toBeNull()
  })

  it('4096 caracteres justos valen', () => {
    expect(problemaDeLaCsp('x'.repeat(4096))).toBeNull()
  })

  it('el tope se cuenta en bytes, que es lo más estricto que puede hacer bash', () => {
    // 2.049 «é» son 2.049 caracteres pero 4.098 bytes.
    expect(problemaDeLaCsp('é'.repeat(2049))).toMatch(/4096/)
  })

  it('los de control C1 también cuentan, como en [[:cntrl:]] con UTF-8', () => {
    expect(problemaDeLaCsp('default-src a\u0085b')).toMatch(/control/)
  })
})

describe('el fichero', () => {
  it('sin CSP no aparece ninguna de las dos claves', () => {
    const o = leer(generar(info({}), null))
    expect(o).not.toHaveProperty('csp')
    expect(o).not.toHaveProperty('csp_report_only')
  })

  it('con una buena, sale recortada', () => {
    const o = leer(generar(info({}), null, { csp: "  default-src 'self'  " }))
    expect(o.csp).toBe("default-src 'self'")
    expect(o).not.toHaveProperty('csp_report_only')
  })

  it('«sólo informar» sólo se escribe junto a una política', () => {
    expect(leer(generar(info({}), null, { csp: "default-src 'self'", cspSoloInforme: true })).csp_report_only).toBe(true)
    expect(leer(generar(info({}), null, { csp: '', cspSoloInforme: true }))).not.toHaveProperty('csp_report_only')
  })

  it('una que Orbit rechazaría no se emite, y se dice por qué', () => {
    const g = generar(info({}), null, { csp: "default-src 'self';", cspSoloInforme: true })
    const o = leer(g)
    expect(o).not.toHaveProperty('csp')
    expect(o).not.toHaveProperty('csp_report_only')
    expect(g.huecos.join(' ')).toMatch(/«;»/)
  })

  it('una redirección no lleva CSP: no sirve páginas', () => {
    const g = generar(info({ type: 'redirect' }), null, { csp: "default-src 'self'" })
    expect(leer(g)).not.toHaveProperty('csp')
    expect(g.huecos.join(' ')).toMatch(/redirección/)
  })
})

describe('la versión', () => {
  it('la CSP es de la 1.5.1', () => {
    expect(sabe('1.5.0', 'csp')).toBe(false)
    expect(sabe('1.5.1', 'csp')).toBe(true)
    expect(sabe('1.6.0', 'csp')).toBe(true)
  })

  it('con un servidor anterior se avisa de que la ignoraría', async () => {
    const r = render(OrbitJson, { app: 'web', info: info({}), entorno: null, version: '1.4.9' })
    const campo = r.getByLabelText('Política (opcional)') as HTMLInputElement
    await fireEvent.input(campo, { target: { value: "default-src 'self'" } })
    expect(r.container.textContent).toMatch(/Orbit 1\.4\.9/)
    expect(r.container.textContent).toMatch(/se ignora/)
  })

  it('una política mala se marca en el campo', async () => {
    const r = render(OrbitJson, { app: 'web', info: info({}), entorno: null, version: '1.5.1' })
    const campo = r.getByLabelText('Política (opcional)') as HTMLInputElement
    await fireEvent.input(campo, { target: { value: 'default-src "self"' } })
    expect(campo.getAttribute('aria-invalid')).toBe('true')
    expect(r.container.querySelector('pre')!.textContent).not.toMatch(/csp/)
  })
})
