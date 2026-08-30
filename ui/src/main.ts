import { mount } from 'svelte'
import './estilos/tokens.css'
import './estilos/estado.css'
import './estilos/base.css'
import App from './App.svelte'

export default mount(App, { target: document.getElementById('app')! })
