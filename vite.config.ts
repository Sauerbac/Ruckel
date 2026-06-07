import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// @tauri-apps/cli sets this when running on a physical device / LAN host.
const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte(), tailwindcss()],

  // Tauri expects a fixed port and manages the window/console itself.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    // Tauri's Rust sources are watched by cargo, not Vite.
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
