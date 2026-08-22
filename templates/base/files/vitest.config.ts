import { defineConfig } from "vitest/config";

/**
 * Test configuration.
 *
 * This config stands alone rather than extending `vite.config.ts`, so the app's
 * plugins (React, the TanStack Router generator) do not run during tests. The
 * environment is Node because the tests a fresh project ships with cover
 * command-line tooling under `scripts/`. Component tests need a DOM: install
 * `jsdom` plus a DOM testing library and set `environment: "jsdom"` when you
 * write the first one.
 */
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.ts"],
  },
});
