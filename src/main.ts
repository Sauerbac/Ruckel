import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import Gallery from './Gallery.svelte'

// Dev-only gallery harness (F1): `?gallery` mounts the component gallery instead
// of the app. Stripped from production builds — the guard is statically false.
const useGallery = import.meta.env.DEV && location.search.includes('gallery')

const app = mount(useGallery ? Gallery : App, {
  target: document.getElementById('app')!,
})

export default app
