// @ts-check
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://proxai-docs.vercel.app",
  vite: {
    plugins: [tailwindcss()],
    resolve: {
      alias: {
        "@components": new URL("./src/components", import.meta.url).pathname,
      },
    },
  },
  integrations: [
    react(),
    starlight({
      title: "ProxAI",
      defaultLocale: "en",
      locales: {
        en: { label: "English", lang: "en" },
        zh: { label: "中文", lang: "zh-CN" },
      },
      customCss: ["./src/styles/proxai.css"],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/VIDLG/proxai",
        },
      ],
      sidebar: [
        {
          label: "Using ProxAI",
          translations: { zh: "使用 ProxAI" },
          items: [
            { label: "Overview", translations: { zh: "概览" }, link: "/" },
            { slug: "quick-start" },
            { slug: "configuration" },
            { slug: "routing-and-providers" },
            { slug: "observability" },
            { slug: "troubleshooting" },
          ],
        },
        {
          label: "Protocol Guide",
          translations: { zh: "协议指南" },
          items: [
            { slug: "protocols-overview" },
            { slug: "protocol-openai-responses" },
            { slug: "protocol-openai-chat-completions" },
            { slug: "protocol-anthropic-messages" },
            { slug: "streaming-behavior" },
          ],
        },
        {
          label: "Developer Guide",
          translations: { zh: "开发者指南" },
          items: [
            { slug: "architecture" },
            { slug: "protocol-conversion" },
            { slug: "streaming-internals" },
            { slug: "error-handling-internals" },
          ],
        },
        {
          label: "Reference",
          translations: { zh: "参考" },
          items: [
            { slug: "reference/configuration-example" },
            { slug: "reference/cli" },
            { slug: "reference/defaults-and-limits" },
            { slug: "reference/protocols" },
            { slug: "reference/error-responses" },
            { slug: "reference/behavior-contracts" },
          ],
        },
      ],
      editLink: {
        baseUrl:
          "https://github.com/VIDLG/proxai/edit/main/site/src/content/docs/",
      },
    }),
  ],
});
