import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
const isCi = !!process.env.CI;
const isWindows = process.platform === "win32";

/** sass-embedded can leave handles open on Windows; exit explicitly in CI. */
function ciForceExitPlugin(): Plugin {
  return {
    name: "ci-force-exit",
    closeBundle() {
      if (isCi) {
        setImmediate(() => process.exit(0));
      }
    },
  };
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), ...(isCi ? [ciForceExitPlugin()] : [])],

  optimizeDeps: {
    include: ["mqtt", "socket.io-client"],
  },

  css: {
    preprocessorOptions: {
      scss: {
        api: "modern-compiler",
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    minify: isCi ? "esbuild" : false,
    sourcemap: false,
    reportCompressedSize: !isCi,
    watch: false,
    rollupOptions: isCi
      ? {
          // Windows runners deadlock more often with parallel Rollup I/O.
          maxParallelFileOps: isWindows ? 1 : 2,
        }
      : undefined,
  },
  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
