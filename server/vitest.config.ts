import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
    testTimeout: 30_000,
    hookTimeout: 30_000,
    // Miniflare binds a real port per harness; keep instances from colliding.
    fileParallelism: false,
  },
});
