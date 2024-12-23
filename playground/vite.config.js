import { defineConfig } from 'vite'
import wasmPack from 'vite-plugin-wasm-pack'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  plugins: [
    wasm(),
    wasmPack(['../hoddor'])
  ],
  worker: {
    format: 'es',
    plugins: [wasm()]
  },
  server: {
    fs: {
      allow: ['../']
    }
  },
  build: {
    target: 'esnext',
    minify: false
  }
})