import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    fs: { allow: ['.', '../..'] },
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:5580',
        changeOrigin: true,
        ws: true,
        timeout: 0,
      },
    },
  },
})
