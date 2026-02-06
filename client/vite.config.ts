import { defineConfig } from "vite";
import { writeFileSync } from "fs";

const buildTime = Date.now().toString();

export default defineConfig({
  define: {
    __BUILD_TIME__: JSON.stringify(buildTime),
  },
  server: {
    port: 3000,
    proxy: {
      "/ws": {
        target: "ws://localhost:9001",
        ws: true,
      },
    },
  },
  build: {
    chunkSizeWarningLimit: 2000,
  },
  plugins: [
    {
      name: "write-version-json",
      writeBundle() {
        writeFileSync("dist/version.json", JSON.stringify({ t: buildTime }));
      },
    },
  ],
});
