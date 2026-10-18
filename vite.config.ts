import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 期望一个固定端口的开发服务器，且失败时不要回退到其它端口
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue()],
  // Tauri CLI 会自己打印日志，屏蔽 Vite 的清屏避免覆盖
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // src-tauri 由 cargo 自己监听，避免 Vite 重复触发
      ignored: ["**/src-tauri/**"],
    },
  },
});
