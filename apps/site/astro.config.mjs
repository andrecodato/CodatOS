// Site estático (ADR-0008): Astro + Tailwind, sem servidor.
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

export default defineConfig({
  vite: { plugins: [tailwindcss()] },
});
