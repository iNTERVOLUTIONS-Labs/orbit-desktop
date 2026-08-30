import { mount } from 'svelte'
import './estilos/tokens.css'
import './estilos/estado.css'
import './estilos/base.css'
import Galeria from './Galeria.svelte'

export default mount(Galeria, { target: document.getElementById('app')! })
