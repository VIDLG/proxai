# ProxAI Site

ProxAI's documentation site is built with **Astro + Starlight** and is deployed from `site/` to **GitHub Pages** at <https://vidlg.github.io/proxai/>.

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

## Deploy to GitHub Pages

The site is built and deployed by the **Site** workflow at `.github/workflows/site.yml`. On every push to `main` that touches `site/**` or the workflow file itself, it:

1. Installs pnpm + Node and runs `pnpm run build` from `site/`.
2. Uploads `site/dist` as a GitHub Pages artifact.
3. Deploys the artifact to GitHub Pages.

`astro.config.mjs` sets `site: "https://vidlg.github.io"` and `base: "/proxai/"` so all generated URLs are prefixed with the repo subpath. Pushes to other branches and pull requests do not deploy; use `workflow_dispatch` to trigger a manual deploy from a branch.

### One-time repository setup

Before the first deployment, enable Pages in the repo settings:

- **Settings → Pages → Build and deployment → Source**: `GitHub Actions`.

No further configuration is needed; the workflow handles the rest.

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
├── astro.config.mjs            # Starlight integration, navigation, site/base for Pages
├── package.json
└── justfile
```

## Tooling

- Node.js and pnpm are managed by root `pixi.toml`.
- Root `justfile` declares `mod site`; run site tasks as `just site <recipe>`.
- `site/justfile` owns site-local recipes and runs with `site/` as cwd.
- `pnpm-lock.yaml` is committed so CI and local installs use identical dependencies. Update it with `pnpm install` whenever `package.json` changes.
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
