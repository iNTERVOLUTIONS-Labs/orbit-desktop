# Distribuir

> Qué hace falta para publicar Orbit Desktop, qué está hecho, y qué está a
> propósito sin hacer.

## 0. Dónde está esto

La fase 5 se parte en dos mitades que no se parecen en nada:

| | Se puede hoy | Estado |
|---|---|---|
| **Compilar** en Linux, macOS y Windows | sí | hecho, y comprobado ejecutándolo |
| **Firmar**, notarizar y actualizar | no: necesita cuentas de Apple y Microsoft | documentado aquí, **sin escribir** |

La segunda mitad no está escrita a medias, y es deliberado. Un workflow con tres
secretos vacíos y un `continue-on-error` no es media solución: es una que sale en
verde sin haber hecho nada, y el día que existan las credenciales nadie
comprobaría que de verdad funciona porque «ya estaba puesto».

Lo mismo con el actualizador. Enchufar `tauri-plugin-updater` hoy sería añadir
una dependencia **al proceso que sostiene las claves SSH del usuario** —que es el
argumento con el que se descartó Electron— configurada con una clave pública que
no existe y apuntando a un endpoint que tampoco. Tres marcadores de posición en
el camino por el que entra código nuevo a la máquina de alguien. Se añade cuando
haya clave, y no antes.

## 1. Lo que ya está: compilar en las tres

`.github/workflows/paquetes.yml`. Se lanza a mano (`workflow_dispatch`) o al
etiquetar una versión, y **no en cada push**: son tres runners y dos de ellos
cuestan varias veces lo que un `ubuntu-latest`.

Produce, sin firmar:

| Plataforma | Paquetes | Por qué ésos |
|---|---|---|
| Linux | `.deb`, `.AppImage` | El `.deb` declara sus dependencias —`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `openssh-client`— porque sin ellas instala y **la ventana sale en blanco**, que parece un defecto de la aplicación y no una dependencia que falta |
| macOS | `.dmg`, `.app` | El `.dmg` porque un `.app` suelto en una descarga es una carpeta que macOS trata como sospechosa |
| Windows | NSIS `.exe` | Instalación **por usuario**: no pide permisos de administrador, y esta aplicación no los necesita para nada. Lo único elevado que ocurre pasa en el otro extremo del `ssh` |

El cliente de terminal se compila en una matriz aparte, y su fallo diría algo
distinto: no depende de Tauri ni de WebKitGTK, así que si rompe es del núcleo.

### Lo que pesan, medido

| Paquete | Tamaño |
|---|---|
| `Orbit Desktop_0.1.0_amd64.deb` | **1,8 MB** |
| `Orbit Desktop_0.1.0_aarch64.dmg` | **1,9 MB** · sólo Apple Silicon, [a propósito](#macos-sólo-apple-silicon-y-es-una-decisión) |
| `Orbit Desktop_0.1.0_x64-setup.exe` | **1,4 MB** |
| `Orbit Desktop_0.1.0_amd64.AppImage` | 78 MB |

**Ése es el argumento con el que se eligió Tauri sobre Electron, por fin
medido.** Los tres primeros pesan lo que pesan porque usan el motor web **del
sistema** —WebView2, WKWebView, WebKitGTK— en vez de llevarse uno dentro; una
aplicación de Electron equivalente ronda los 85 MB *por plataforma*. El AppImage
es el único grande, y por definición: existe justo para no depender de nada de
la máquina.

### macOS: sólo Apple Silicon, y es una decisión

`macos-latest` es ARM, así que el `.dmg` es `aarch64` y **un Mac con Intel no
tiene paquete**. Está decidido así: no se compila universal.

Se podría —`--target universal-apple-darwin`, compilando las dos arquitecturas—
y cuesta el doble de tiempo de runner y un binario que pesa el doble para que
cada máquina use la mitad. A cambio de cubrir unos Mac que Apple dejó de vender
en 2023 y a los que ya no lleva macOS nuevo.

Si algún día hace falta, es un cambio de una línea en la matriz y este párrafo
explica qué se está deshaciendo.

### Un hueco conocido

**El instalador de Windows no trae WebView2.** Lo instala si falta,
descargándolo —que es lo que hace su modo por defecto— así que una máquina sin
conexión y sin WebView2 se queda a medias. La alternativa es empotrarlo, y son
unos 150 MB frente a los 1,4 de ahora. Se deja como está: una máquina donde se
despliegan webs por SSH tiene conexión por definición.

## 2. Lo que falta, secreto a secreto

### 2.1 macOS · firma y notarización

Sin esto, quien descargue el `.dmg` se encuentra con **«no se puede abrir porque
Apple no puede comprobar que no contenga software malicioso»** y la única salida
es el botón derecho → Abrir, que es exactamente el gesto que hay que enseñar a
*no* hacer.

| Secreto | Qué es | De dónde sale |
|---|---|---|
| `APPLE_CERTIFICATE` | El `.p12` del certificado *Developer ID Application*, en base64 | Cuenta de Apple Developer (99 $/año). Se genera desde Xcode o desde el portal |
| `APPLE_CERTIFICATE_PASSWORD` | La contraseña del `.p12` | La que se puso al exportarlo |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: NOMBRE (TEAMID)` | `security find-identity -v -p codesigning` |
| `APPLE_ID` | El correo de la cuenta | — |
| `APPLE_PASSWORD` | Una **contraseña específica de aplicación**, no la de la cuenta | appleid.apple.com → Iniciar sesión y seguridad |
| `APPLE_TEAM_ID` | Diez caracteres | El portal de Apple Developer |

Notarizar es un paso aparte de firmar: se sube el paquete a Apple, se espera
—minutos, a veces más— y se le grapa el resultado. Sin el grapado, la primera
apertura sin conexión vuelve a fallar.

### 2.2 Windows · firma

Sin esto, SmartScreen enseña **«Windows protegió tu PC»** y esconde el botón de
ejecutar detrás de «Más información». Con un certificado normal el aviso
desaparece **después de cierta reputación**, no desde el primer día; con uno EV
desaparece antes y cuesta bastante más.

| Secreto | Qué es |
|---|---|
| `WINDOWS_CERTIFICATE` | El `.pfx` en base64 |
| `WINDOWS_CERTIFICATE_PASSWORD` | Su contraseña |

Desde 2023 las autoridades exigen que la clave viva en un HSM o en un token
físico, así que «el `.pfx` en un secreto de GitHub» ya no es una opción para un
certificado nuevo: hay que usar un servicio de firma en la nube (Azure Trusted
Signing, SSL.com eSigner…) y el workflow cambia de forma. **Conviene decidir eso
antes de comprar el certificado, no después.**

El paso a paso —qué pedir, a quién, qué papeles hacen falta y qué tarda cada
cosa— está en [CERTIFICADO-WINDOWS.md](CERTIFICADO-WINDOWS.md).

### 2.3 El actualizador · minisign

Tauri firma cada actualización con **minisign** y comprueba la firma con una
clave pública **empotrada en el binario**. Eso es lo que hace que el canal de
actualización no dependa de que el servidor de descargas sea de fiar: aunque
alguien lo controle, sin la clave privada no puede publicar nada que se instale.

```bash
# La privada NUNCA entra en el repositorio ni sale de un gestor de secretos.
cargo tauri signer generate -w ~/.tauri/orbit-desktop.key
```

| Secreto | Qué es |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | La clave privada |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Su frase de paso |

Y la **pública** va en `tauri.conf.json`, en claro, porque para eso es pública.

Dos decisiones que conviene tomar a la vez que se genera la clave:

**Dónde se publica el manifiesto.** Las releases de GitHub valen y no cuestan
nada. Si algún día se sirve desde un dominio propio, la clave sigue siendo la que
protege: el endpoint no tiene que ser de fiar.

**Si la actualización es automática o se pregunta.** La recomendación es
**preguntar**. Esta aplicación tiene sesiones SSH multiplexadas abiertas y puede
tener un despliegue de tres minutos en marcha; reiniciarse sola en mitad de eso
es la peor cosa que puede hacer. El `AvisoDeCierre` que ya existe en la interfaz
está escrito con ese criterio y esto es la misma regla.

### 2.4 Linux · nada

`.deb` y `.AppImage` no se firman para instalarse. Si algún día hay un repositorio
apt propio, ahí sí hace falta una clave GPG — y sería una decisión aparte.

## 3. El orden en que conviene hacerlo

1. **Pedir las cuentas.** Apple y Microsoft tardan, y a veces piden papeles de la
   empresa. Es lo único que no se puede acelerar, y por eso `ROADMAP.md` dice que
   se piden en la fase 1 aunque se usen en la quinta.
2. **Decidir cómo se firma en Windows** (token, HSM o servicio en la nube) antes
   de comprar nada: cambia el workflow.
3. **Generar la clave de minisign** y guardarla donde se guarden las cosas serias.
   Esta no la da nadie: se genera y se pierde una sola vez.
4. **Entonces** escribir la mitad que falta, y probarla publicando una versión de
   prueba y actualizando desde ella. Una actualización que no se ha probado
   actualizando no está probada.

## 4. Mientras tanto

Se puede compilar y repartir sin firmar, diciendo lo que hay: los avisos de
Gatekeeper y de SmartScreen aparecerán, y decir de antemano que van a aparecer y
por qué es mejor que dejar que alguien los descubra y se lo tome —con razón— como
una señal de que algo va mal.

En Linux no aparece ninguno, así que el `.deb` y el `.AppImage` se pueden repartir
hoy tal cual.
