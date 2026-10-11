import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// The app is a Tauri window: fixed port for `tauri dev`, relative paths for the built files
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: './',
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  server: { port: 1420, strictPort: true },
  build: { outDir: 'dist', target: 'es2022', chunkSizeWarningLimit: 2000 },
})
