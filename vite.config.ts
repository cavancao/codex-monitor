import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Windows 会锁定正在执行的 Rust 构建脚本；前端监听必须排除 Cargo 产物。
    watch: { ignored: ["**/src-tauri/target/**"] }
  },
  envPrefix: ["VITE_", "TAURI_ENV_"]
});
