# ProxAI Site

ProxAI's documentation site is built with **Astro + Starlight** and is intended to be deployed from `site/` on **Vercel**.

`site/` is the single source for publishable documentation. Legacy Markdown docs under `docs/` have been removed to avoid dual-source drift.

## Local development

From the repository root:

```bash
just site install
just site dev
```

Or from `site/` directly:

```bash
just install
just dev
```

Open http://localhost:4321.

## Build

From the repository root:

```bash
just site build
just site start
```

Or from `site/` directly:

```bash
just build
just start
```

`just site start` previews the production build.

## Deploy to Vercel

1. Import the `proxai` repository in Vercel.
2. Set **Root Directory** to `site`.
3. Use the default Astro/Vercel build detection, or set:
   - **Install Command**: `pnpm install`
   - **Build Command**: `pnpm run build`
   - **Output Directory**: `dist`
4. Deploy.

Every push to the production branch can deploy the site, and pull requests can receive preview URLs.

## Structure

```text
site/
├── src/
│   └── content/
│       └── docs/
│           ├── en/             # English documentation
│           │   ├── index.mdx
│           │   └── *.mdx
│           └── zh/             # Chinese documentation
│               ├── index.mdx
│               └── *.mdx
├── astro.config.mjs            # Starlight integration and navigation
├── package.json
└── justfile
```

## Tooling

- Node.js and pnpm are managed by root `pixi.toml`.
- Root `justfile` declares `mod site`; run site tasks as `just site <recipe>`.
- `site/justfile` owns site-local recipes and runs with `site/` as cwd.
- `pnpm-lock.yaml` is intentionally ignored for now; regenerate it locally as needed.

## Documentation rules

- Publishable docs live in `site/src/content/docs/{en,zh}/`.
- New user-facing docs should be added in both English and Chinese.
- Keep page slugs paired across languages where practical, for example:
  - `site/src/content/docs/en/architecture.mdx`
  - `site/src/content/docs/zh/architecture.mdx`
- Navigation is configured in `site/astro.config.mjs`.
- Prefer Starlight-native components (`Aside`, `Card`, `CardGrid`, `Steps`, `Tabs`, `FileTree`) before custom components.
- Prefer Mermaid diagrams over images when a diagram can be text-based.
