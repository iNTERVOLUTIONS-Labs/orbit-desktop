# De dónde salen estos iconos

Del **logotipo de Orbit**: `assets/logo-mark.svg` del repositorio
[`iNTERVOLUTIONS-Labs/orbit`](https://github.com/iNTERVOLUTIONS-Labs/orbit),
MIT, © Intervolutions. Mismo dueño y misma familia de producto: este cliente es
la ventana de esa herramienta, y darle una marca inventada aparte habría sido
inventarse un producto distinto.

Se rinde el SVG a 1024×1024 y de ahí sale el resto con `cargo tauri icon`.

## Por qué no estaban antes

Había un solo `icon.png` de **32×32 y 104 bytes**: un marcador de posición. En
Linux y macOS eso pasa desapercibido porque el empaquetado lo escala y sigue
adelante; **en Windows no**, porque `tauri-build` necesita un `icon.ico` para
generar el recurso del ejecutable y aborta la compilación si falta.

O sea que el proyecto no podía compilarse para Windows y nadie lo sabía, porque
nunca se había intentado. Lo dijo la primera tanda de la matriz:

```
icons/icon.ico not found; required for generating a Windows Resource file
```

## Qué NO hay aquí

`cargo tauri icon` genera además `android/` e `ios/`. Se borran: el
`ROADMAP.md` declara la versión móvil **fuera de alcance**, y por un motivo que
no es de esfuerzo — un cliente que sostiene las claves SSH de alguien en un
aparato que se pierde es exactamente lo que este proyecto no hace. Dejar sus
iconos aquí insinuaría lo contrario.

## Si algún día se rehace

El fuente es el SVG del otro repositorio, no el PNG de esta carpeta. Regenerarlo
desde el PNG sería escalar una escala.
