// @ts-check
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://vidlg-proxai.netlify.app",
  vite: {
    plugins: [tailwindcss()],
    resolve: {
      alias: {
        "@components": new URL("./src/components", import.meta.url).pathname,
      },
    },
  },
  integrations: [
    react({
      include: ["**/src/components/**/*.{jsx,tsx}"],
      babel: {
        plugins: ["babel-plugin-react-compiler"],
      },
    }),
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
            { slug: "using" },
            { slug: "using/quick-start" },
            { slug: "using/install-and-upgrade" },
            { slug: "using/client-integration" },
            { slug: "using/zed-integration" },
            { slug: "using/provider-setup" },
            { slug: "using/streaming-and-tools" },
            { slug: "using/privacy-and-local-data" },
            { slug: "using/known-limitations" },
            { slug: "using/debugging-workflow" },
            { slug: "using/recipes" },
            { slug: "using/configuration" },
            { slug: "using/routing-and-providers" },
            { slug: "using/observability" },
            { slug: "using/troubleshooting" },
          ],
        },
        {
          label: "Protocol Guide",
          translations: { zh: "协议指南" },
          items: [
            { slug: "protocol" },
            { slug: "protocol/choosing-a-protocol" },
            { slug: "protocol/lossiness-and-fidelity" },
            {
              label: "OpenAI Responses",
              translations: { zh: "OpenAI Responses" },
              items: [
                { slug: "protocol/openai-responses" },
                { slug: "protocol/openai-responses/interaction-example" },
              ],
            },
            {
              label: "OpenAI Chat Completions",
              translations: { zh: "OpenAI Chat Completions" },
              items: [
                { slug: "protocol/openai-chat-completions" },
                {
                  slug: "protocol/openai-chat-completions/interaction-example",
                },
              ],
            },
            {
              label: "Anthropic Messages",
              translations: { zh: "Anthropic Messages" },
              items: [
                { slug: "protocol/anthropic-messages" },
                { slug: "protocol/anthropic-messages/interaction-example" },
              ],
            },
            { slug: "protocol/streaming-behavior" },
          ],
        },
        {
          label: "Developer Guide",
          translations: { zh: "开发者指南" },
          items: [
            { slug: "developer" },
            {
              label: "Architecture",
              translations: { zh: "架构" },
              items: [
                {
                  label: "Architecture",
                  translations: { zh: "架构" },
                  link: "/developer/architecture/",
                },
                { slug: "developer/architecture/request-lifecycle" },
                { slug: "developer/architecture/module-boundaries" },
                { slug: "developer/architecture/config-flow" },
                { slug: "developer/architecture/error-flow" },
              ],
            },
            {
              label: "Protocol Conversion",
              translations: { zh: "协议转换" },
              items: [
                { slug: "developer/protocol-conversion" },
                { slug: "developer/protocol-conversion/message-placement" },
                { slug: "developer/protocol-conversion/refusal-and-status" },
                {
                  slug: "developer/protocol-conversion/anthropic-messages-to-openai-responses",
                },
                {
                  slug: "developer/protocol-conversion/openai-chat-completions-to-anthropic-messages",
                },
                {
                  slug: "developer/protocol-conversion/openai-responses-to-anthropic-messages",
                },
                { slug: "developer/protocol-conversion/sdk-alignment" },
                { slug: "developer/protocol-conversion/streaming-identifiers" },
              ],
            },
            { slug: "developer/streaming-internals" },
            { slug: "developer/error-handling-internals" },
            { slug: "developer/where-to-change-code" },
            { slug: "developer/test-map" },
            { slug: "developer/contributor-checklist" },
            { slug: "developer/docs-maintenance" },
          ],
        },
        {
          label: "Reference",
          translations: { zh: "参考" },
          items: [
            { slug: "reference" },
            { slug: "reference/configuration" },
            { slug: "reference/cli" },
            { slug: "reference/defaults-and-limits" },
            { slug: "reference/protocols" },
            { slug: "reference/compatibility-matrix" },
            { slug: "reference/status-and-stop-reasons" },
            { slug: "reference/route-matching" },
            { slug: "reference/capture-phases" },
            { slug: "reference/environment-and-files" },
            { slug: "reference/error-responses" },
            { slug: "reference/behavior-contracts" },
            { slug: "reference/glossary" },
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
