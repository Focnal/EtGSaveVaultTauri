import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 开发时通过 devUrl 访问该端口，构建时把 dist/ 内嵌进二进制
export default defineConfig({
  plugins: [vue()],
  // Tauri 使用自定义协议加载资源，必须用相对路径
  base: './',
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // src-tauri 由 cargo 自己监听，避免 vite 重复触发
      ignored: ['**/src-tauri/**']
    }
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'chrome110',
    minify: 'esbuild',
    cssMinify: true,
    chunkSizeWarningLimit: 900
  }
})
