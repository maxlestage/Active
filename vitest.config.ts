import { defineConfig } from "vitest/config";

// Library tests only (site/, bench/ and examples/ have their own).
export default defineConfig({ test: { include: ["test/**/*.test.{ts,tsx}"] } });
