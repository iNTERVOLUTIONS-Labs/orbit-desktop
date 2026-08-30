/// <reference types="vite/client" />

// `?raw` de Vite: trae el fichero como texto en vez de parsearlo. Se usa para
// el NDJSON de las muestras, que no es JSON —es una línea de JSON por línea— y
// que el lector tiene que ver crudo, igual que lo ve cuando llega del servidor.
declare module '*?raw' {
  const contenido: string
  export default contenido
}
