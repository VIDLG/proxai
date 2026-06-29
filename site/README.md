# ProxAI Site

ProxAI's documentation site is built with **Astro + Starlight** and is intended to be deployed from `site/` on **Netlify**.

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

## Build and checks

From the repository root:

```bash
just site build
just site check
just site start
```

Or from `site/` directly:

```bash
just build
just check
just check-docs
just start
```

`just site check` builds the site and runs documentation consistency checks. `just site start` previews the production build.

## Deploy to Netlify

`site/netlify.toml` defines the site-local build settings:

```toml
[build]
command = "pnpm run build"
publish = "dist"
```

When importing the repository in Netlify, set **Base directory** to `site`. Netlify will then read `site/netlify.toml`, run the build from `site/`, and publish `site/dist`.

Every push to the production branch can deploy the site, and pull requests can receive deploy previews.

## Structure

```text
site/
├── src/
│   └── content/
│       └── docs/
│           ├── en/             # English documentation
│           │   ├── index.mdx
│           │   ├── using/
│           │   ├── protocol/
│           │   ├── developer/
│           │   └── reference/
│           └── zh/             # Chinese documentation
│               ├── index.mdx
│               ├── using/
│               ├── protocol/
│               ├── developer/
│               └── reference/
├── astro.config.mjs            # Starlight integration and navigation
├── netlify.toml                # Netlify build config for site/ as base directory
├── package.json
└── justfile
```

## Tooling

- Node.js and pnpm are managed by root `pixi.toml`.
- Root `justfile` declares `mod site`; run site tasks as `just site <recipe>`.
- `site/justfile` owns site-local recipes and runs with `site/` as cwd.
- `pnpm-lock.yaml` is intentionally ignored for now; regenerate it locally as needed.
- `just check-docs` enforces paired pages, sidebar slugs, required frontmatter, internal links and anchors, hub coverage, heading quality, old-path regressions, Markdown table leftovers, and developer `noindex` rules.

## Documentation rules

- Publishable docs live in `site/src/content/docs/{en,zh}/`.
- Keep content ownership clear:
  - `using/` owns task walkthroughs: run, configure, route, observe, troubleshoot.
  - `protocol/` owns wire behavior, request/response shapes, streaming expectations, and interaction examples.
  - `developer/` owns source-level architecture and maintainer guidance; internal pages should use `robots: noindex`.
  - `reference/` owns exact values, defaults, phase names, protocol names, behavior contracts, and glossary terms.
- New user-facing docs should be added in both English and Chinese.
- Keep page slugs paired across languages where practical, for example:
  - `site/src/content/docs/en/developer/architecture.mdx`
  - `site/src/content/docs/zh/developer/architecture.mdx`
- Navigation is configured in `site/astro.config.mjs`.
- Prefer Starlight-native components (`Aside`, `Card`, `CardGrid`, `Steps`, `Tabs`, `FileTree`) before custom components.
- Prefer Mermaid diagrams over images when a diagram can be text-based.
