# Conseguir el certificado de firma para Windows

> Paso a paso, con lo que cuesta y lo que tarda cada cosa. Está en un documento
> aparte de [DISTRIBUCION.md](DISTRIBUCION.md) porque **no es trabajo de
> programar**: es papeleo con un proveedor, y lo hace una persona con acceso a
> los datos de la empresa.

## Antes de nada: lo que ha cambiado, y por qué importa

Durante años esto era simple: comprabas un certificado, te bajabas un `.pfx`, lo
metías en un secreto de GitHub y firmabas en el CI.

**Eso ya no se puede.** Desde junio de 2023 las reglas del CA/Browser Forum
obligan a que la clave privada de un certificado de firma de código viva en
hardware certificado —un HSM o un token USB— y no en un fichero. Ninguna
autoridad emite ya un `.pfx` descargable para un certificado nuevo.

Eso deja tres caminos, y **elegir cuál antes de pagar es lo único importante de
este documento**, porque cambia el precio, el plazo y la forma del workflow.

| | Token físico | Firma en la nube | Azure Trusted Signing |
|---|---|---|---|
| Qué llegas a tener | Un USB por correo | Una cuenta en un servicio | Un recurso de Azure |
| Sirve para CI | **Mal**: hay que enchufarlo a una máquina | Sí | Sí, es lo que mejor encaja |
| Coste anual aproximado | el del certificado | el del certificado + el servicio | de los más baratos, por uso |
| Plazo | semanas (envío incluido) | días–semanas | días, si la empresa se valida sola |

> Los precios cambian y no me los invento aquí: míralos en el proveedor el día
> que vayas. Lo que no cambia es la forma de cada opción.

**Recomendación para Intervolutions: Azure Trusted Signing**, y si no encaja,
un servicio de firma en la nube. El token físico se descarta por un motivo
concreto y no de comodidad: firmar en el CI con un USB significa tener una
máquina propia encendida con el token puesto, y eso es una máquina más que
mantener y un token que alguien puede llevarse.

**El único requisito que puede tumbarlo:** Azure Trusted Signing pide, para su
nivel normal, que la organización lleve **tres años o más** de existencia
verificable. Si Intervolutions no llega, hay un nivel para organizaciones nuevas
con condiciones distintas, o se va a la segunda columna. Compruébalo **antes** de
empezar el resto.

---

## Paso 0 · Reunir lo que te van a pedir (1 día)

Da igual el camino: todos validan que la empresa existe. Ten esto a mano antes
de empezar, porque el reloj de la validación corre mientras lo buscas.

- [ ] **Nombre legal exacto** de la empresa, tal y como está registrado. No el
      comercial. Si en el registro pone «INTERVOLUTIONS S.L.», eso es lo que
      tiene que decir en todas partes, con el mismo espaciado.
- [ ] **CIF/NIF** y país de registro.
- [ ] **Dirección física** registrada. Un apartado de correos no vale.
- [ ] **Un teléfono de la empresa que aparezca en una fuente pública
      independiente**: el registro mercantil, Dun & Bradstreet, o un directorio
      telefónico reconocido. Esto es lo que más gente atasca — no basta con el
      teléfono de tu web.
- [ ] **Un correo del dominio de la empresa** (no gmail).
- [ ] **Número D-U-N-S** si lo tenéis. Si no, se pide gratis en Dun &
      Bradstreet y **tarda semanas**: si no lo tenéis, pedidlo hoy aunque el
      resto lo hagáis en un mes.

## Paso 1 · Decidir OV o EV (10 minutos, y decide bastante)

|  | OV (normal) | EV (extendida) |
|---|---|---|
| SmartScreen | El aviso **«Windows protegió tu PC»** sigue apareciendo hasta que el instalador acumula reputación: descargas y ejecuciones sin incidentes, semanas | Reputación **desde la primera descarga**: sin aviso |
| Precio | el bajo | bastante más |
| Validación | la de arriba | la de arriba, más estricta |

**Para lo que es este producto, OV basta.** Orbit Desktop no se reparte a
desconocidos por descarga masiva: lo instala gente que ya usa Orbit y a quien se
le puede decir de antemano qué va a ver. Y el aviso desaparece solo con el
tiempo.

EV sólo compensa si algún día se reparte en abierto y el primer contacto con
alguien que no os conoce tiene que ser limpio.

## Paso 2A · Azure Trusted Signing (el camino recomendado)

1. **Suscripción de Azure.** Si no hay ninguna, se crea; el servicio se factura
   por uso.
2. En el portal, crear un recurso **Trusted Signing Account**. Elige la región
   más cercana: la firma es una llamada de red por cada build.
3. Dentro de la cuenta, crear un **Identity Validation** de tipo
   *Public Trust*. Aquí se meten los datos del paso 0.
4. **Esperar.** Microsoft valida a la organización. Es el paso largo y no
   depende de ti; suelen ser días.
5. Cuando esté aprobada, crear un **perfil de certificado** (*Certificate
   Profile*). El nombre del perfil es lo que se pondrá en el workflow.
6. Crear un **registro de aplicación** (App Registration) en Entra ID y darle el
   rol **Trusted Signing Certificate Profile Signer** sobre la cuenta. De ahí
   salen los tres valores que van a los secretos:

   | Secreto de GitHub | De dónde sale |
   |---|---|
   | `AZURE_TENANT_ID` | Entra ID → Overview |
   | `AZURE_CLIENT_ID` | El App Registration |
   | `AZURE_CLIENT_SECRET` | El App Registration → Certificates & secrets |

   Y además, en claro y no como secretos: el **endpoint** de la región, el
   **nombre de la cuenta** y el **nombre del perfil**.

7. Avisar cuando estén los tres secretos puestos, y se escribe la parte del
   workflow. **No antes**: ver el porqué al final.

## Paso 2B · Servicio de firma en la nube (si Azure no encaja)

Los conocidos son **SSL.com eSigner**, **DigiCert KeyLocker** y **Certum** —éste
último suele ser el más barato para una empresa pequeña.

1. Comprar el certificado *Code Signing* **OV** eligiendo la opción de
   almacenamiento en la nube del proveedor, **no** la de token físico. Este es el
   punto donde es fácil equivocarse: si eliges token, te lo mandan por correo y
   ya no sirve para el CI.
2. Pasar la validación con los datos del paso 0. El proveedor llamará al
   teléfono público de la empresa: que alguien lo coja.
3. Cuando esté emitido, sacar las credenciales de API del servicio de firma
   —usuario, contraseña, TOTP o clave— y ponerlas de secretos.
4. Avisar. La forma exacta del workflow depende del proveedor.

## Paso 3 · Lo que cambia en este repositorio

Nada de esto se toca hasta que existan las credenciales. Cuando existan:

- `.github/workflows/paquetes.yml` gana un paso de firma **después** de
  `cargo tauri build`, sólo en el trabajo de Windows.
- No hace falta tocar `tauri.conf.json`: se firma el `.exe` que ya sale.
- Y se prueba **descargando el instalador firmado en un Windows de verdad** y
  mirando que no salga el aviso. Una firma que no se ha comprobado instalando no
  está comprobada.

## Por qué el workflow no está ya escrito esperando

Porque un paso de firma con tres secretos vacíos y un `continue-on-error` sale
en verde sin haber hecho nada, y el día que lleguen las credenciales nadie va a
comprobar que de verdad funciona: estará puesto desde hace meses y parecerá
probado.

Escribirlo cuando haya con qué probarlo cuesta media hora y se sabe que sirve.

---

## Mientras tanto

El instalador **sin firmar funciona perfectamente**. Lo que pasa es que Windows
enseña **«Windows protegió tu PC»** con el botón de ejecutar escondido detrás de
*Más información* → *Ejecutar de todas formas*.

Si se lo pasas a alguien, dile antes que va a salir ese aviso y por qué. Que lo
descubra solo es peor: el aviso está pensado exactamente para que desconfíe, y
hace bien.
