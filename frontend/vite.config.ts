/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: { proxy: { '/api': 'http://127.0.0.1:8088' } },
  build: { outDir: 'dist' },
  test: { environment: 'jsdom', globals: true, setupFiles: './src/setupTests.ts' },
})
