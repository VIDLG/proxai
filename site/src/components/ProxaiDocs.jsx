import {
  AlertTriangle,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  CircleSlash,
  FileCog,
  Gauge,
  GitBranch,
  KeyRound,
  Info,
  Network,
  Route,
  Rocket,
  Settings,
  ShieldAlert,
  Shuffle,
  TerminalSquare,
  Timer,
  Zap,
} from "lucide-react";

const icons = {
  alert: AlertTriangle,
  architecture: Network,
  book: BookOpen,
  check: CheckCircle2,
  config: Settings,
  file: FileCog,
  gauge: Gauge,
  flow: GitBranch,
  info: Info,
  key: KeyRound,
  protocol: Shuffle,
  route: Route,
  rocket: Rocket,
  security: ShieldAlert,
  terminal: TerminalSquare,
  timer: Timer,
  unsupported: CircleSlash,
  zap: Zap,
};

export function ProxaiIcon({ name = "info", size = 18, className = "" }) {
  const Component = icons[name] ?? Info;
  return (
    <Component
      aria-hidden="true"
      className={className}
      size={size}
      strokeWidth={2}
    />
  );
}

export function ProxaiCallout({ type = "note", title, children }) {
  const meta = {
    note: {
      icon: "info",
      label: title ?? "Note",
      accent: "border-s-[var(--sl-color-accent)]",
    },
    tip: {
      icon: "rocket",
      label: title ?? "Tip",
      accent: "border-s-[var(--sl-color-green-high)]",
    },
    caution: {
      icon: "alert",
      label: title ?? "Caution",
      accent: "border-s-[var(--sl-color-orange-high)]",
    },
    success: {
      icon: "check",
      label: title ?? "Success",
      accent: "border-s-[var(--sl-color-green-high)]",
    },
  }[type] ?? {
    icon: "info",
    label: title ?? "Note",
    accent: "border-s-[var(--sl-color-accent)]",
  };

  return (
    <aside
      className={`my-5 rounded-xl border border-[var(--sl-color-gray-5)] border-s-4 ${meta.accent} bg-[var(--sl-color-gray-6)] p-4`}
    >
      <div className="flex items-center gap-2 text-[var(--sl-color-white)]">
        <ProxaiIcon name={meta.icon} />
        <strong>{meta.label}</strong>
      </div>
      <div className="mt-2 [&>*:first-child]:mt-0 [&>*:last-child]:mb-0">
        {children}
      </div>
    </aside>
  );
}

export function FeatureGrid({ items = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 md:grid-cols-2">
      {items.map((item) => (
        <a
          className="group grid grid-cols-[auto_1fr_auto] items-start gap-3 rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4 no-underline transition hover:border-[var(--sl-color-accent)] hover:bg-[var(--sl-color-gray-5)]"
          href={item.href}
          key={item.href ?? item.title}
        >
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-[var(--sl-color-accent-low)] text-[var(--sl-color-accent-high)]">
            <ProxaiIcon name={item.icon} size={20} />
          </span>
          <span className="grid gap-1">
            <strong className="text-[var(--sl-color-white)]">
              {item.title}
            </strong>
            <span className="text-sm leading-6 text-[var(--sl-color-gray-2)]">
              {item.description}
            </span>
          </span>
          <ArrowRight
            className="mt-1 text-[var(--sl-color-gray-3)] transition group-hover:translate-x-0.5 group-hover:text-[var(--sl-color-accent-high)]"
            aria-hidden="true"
            size={18}
          />
        </a>
      ))}
    </div>
  );
}

export function StatusPill({ status }) {
  const normalized = String(status ?? "").toLowerCase();
  const supported =
    normalized.includes("pass") ||
    normalized.includes("conversion") ||
    normalized.includes("转换") ||
    normalized.includes("透传");
  const classes = supported
    ? "border-[var(--sl-color-green-high)]/40 bg-[color-mix(in_srgb,var(--sl-color-green-low),transparent_60%)] text-[var(--sl-color-green-high)]"
    : "border-[var(--sl-color-orange-high)]/40 bg-[color-mix(in_srgb,var(--sl-color-orange-low),transparent_60%)] text-[var(--sl-color-orange-high)]";

  return (
    <span
      className={`inline-flex rounded-full border px-2 py-0.5 text-xs font-semibold ${classes}`}
    >
      {status}
    </span>
  );
}

export function ProtocolMatrix({ rows = [], labels }) {
  const text = {
    inbound: "Inbound protocol",
    provider: "Provider protocol",
    status: "Status",
    ...labels,
  };

  return (
    <div className="my-4 overflow-x-auto rounded-xl border border-[var(--sl-color-gray-5)]">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-[var(--sl-color-gray-6)]">
          <tr>
            <th className="border-b border-[var(--sl-color-gray-5)] px-4 py-3 text-left">
              {text.inbound}
            </th>
            <th className="border-b border-[var(--sl-color-gray-5)] px-4 py-3 text-left">
              {text.provider}
            </th>
            <th className="border-b border-[var(--sl-color-gray-5)] px-4 py-3 text-left">
              {text.status}
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              className="border-b border-[var(--sl-color-gray-6)] last:border-0"
              key={`${row.inbound}-${row.provider}-${row.status}`}
            >
              <td className="px-4 py-3">
                <code>{row.inbound}</code>
              </td>
              <td className="px-4 py-3">
                <code>{row.provider}</code>
              </td>
              <td className="px-4 py-3">
                <StatusPill status={row.status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function PipelineStrip({ steps = [] }) {
  return (
    <ol className="my-5 grid list-none gap-3 p-0">
      {steps.map((step, index) => (
        <li
          className="flex items-start gap-3 rounded-xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-3"
          key={step.title}
        >
          <span className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-[var(--sl-color-accent-high)] text-sm font-extrabold text-[var(--sl-color-black)]">
            {index + 1}
          </span>
          <span>
            <strong className="text-[var(--sl-color-white)]">
              {step.title}
            </strong>
            {step.description ? (
              <small className="mt-1 block text-[var(--sl-color-gray-2)]">
                {step.description}
              </small>
            ) : null}
          </span>
        </li>
      ))}
    </ol>
  );
}

export function ConfigMap({ groups = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 lg:grid-cols-2">
      {groups.map((group) => (
        <section
          className="rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4"
          key={group.title}
        >
          <div className="mb-3 flex items-center gap-2 text-[var(--sl-color-white)]">
            <span className="grid h-8 w-8 place-items-center rounded-lg bg-[var(--sl-color-accent-low)] text-[var(--sl-color-accent-high)]">
              <ProxaiIcon name={group.icon} size={18} />
            </span>
            <strong>{group.title}</strong>
          </div>
          {group.description ? (
            <p className="mb-3 text-sm leading-6 text-[var(--sl-color-gray-2)]">
              {group.description}
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            {group.keys?.map((key) => (
              <code
                className="rounded-md border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-black)] px-2 py-1 text-xs"
                key={key}
              >
                {key}
              </code>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

export function ModuleGrid({ modules = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
      {modules.map((module) => (
        <section
          className="rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4"
          key={module.name}
        >
          <div className="mb-2 flex items-center gap-2 text-[var(--sl-color-white)]">
            <ProxaiIcon name={module.icon ?? "file"} />
            <strong>
              <code>{module.name}</code>
            </strong>
          </div>
          <p className="m-0 text-sm leading-6 text-[var(--sl-color-gray-2)]">
            {module.description}
          </p>
        </section>
      ))}
    </div>
  );
}

export function ConversionPath({ from, via, to }) {
  const items = [from, ...(via ?? []), to].filter(Boolean);
  return (
    <div className="my-5 flex flex-col gap-3 rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4 md:flex-row md:items-center">
      {items.map((item, index) => (
        <div
          className="flex flex-1 items-center gap-3"
          key={`${item.title}-${index}`}
        >
          <div className="min-w-0 flex-1 rounded-xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-black)] p-3">
            <div className="text-sm font-semibold text-[var(--sl-color-white)]">
              {item.title}
            </div>
            {item.description ? (
              <div className="mt-1 text-xs leading-5 text-[var(--sl-color-gray-2)]">
                {item.description}
              </div>
            ) : null}
          </div>
          {index < items.length - 1 ? (
            <ArrowRight
              className="hidden shrink-0 text-[var(--sl-color-accent-high)] md:block"
              size={20}
            />
          ) : null}
        </div>
      ))}
    </div>
  );
}

export function ProtocolCompare({ columns = [], rows = [] }) {
  return (
    <div className="my-4 overflow-x-auto rounded-xl border border-[var(--sl-color-gray-5)]">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-[var(--sl-color-gray-6)]">
          <tr>
            <th className="border-b border-[var(--sl-color-gray-5)] px-4 py-3 text-left">
              {columns[0] ?? "Concept"}
            </th>
            {columns.slice(1).map((col) => (
              <th
                className="border-b border-[var(--sl-color-gray-5)] px-4 py-3 text-left"
                key={col}
              >
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr
              className="border-b border-[var(--sl-color-gray-6)] last:border-0"
              key={`${row[0]}-${index}`}
            >
              <td className="px-4 py-3 align-top font-semibold text-[var(--sl-color-white)]">
                {row[0]}
              </td>
              {row.slice(1).map((cell, cellIndex) => (
                <td
                  className="px-4 py-3 align-top text-[var(--sl-color-gray-2)]"
                  key={cellIndex}
                >
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function ProtocolQuickFacts({
  protocol,
  requestPath,
  codeAreas = [],
  convertsTo = [],
  passThrough = true,
}) {
  return (
    <section className="not-prose my-6 rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4">
      <div className="grid gap-3 md:grid-cols-2">
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
            Protocol value
          </div>
          <div className="mt-1">
            <code className="rounded-md border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-black)] px-2 py-1">
              {protocol}
            </code>
          </div>
        </div>
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
            Request path
          </div>
          <div className="mt-1">
            <code className="rounded-md border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-black)] px-2 py-1">
              {requestPath}
            </code>
          </div>
        </div>
      </div>
      <div className="mt-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
          Main code areas
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          {codeAreas.map((area) => (
            <code
              className="rounded-md border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-black)] px-2 py-1 text-xs"
              key={area}
            >
              {area}
            </code>
          ))}
        </div>
      </div>
      <div className="mt-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
          Conversion targets
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {passThrough ? (
            <span className="inline-flex items-center rounded-full border border-[var(--sl-color-green-high)]/40 bg-[color-mix(in_srgb,var(--sl-color-green-low),transparent_60%)] px-2 py-0.5 text-xs font-semibold text-[var(--sl-color-green-high)]">
              pass-through self
            </span>
          ) : null}
          {convertsTo.map((target) => (
            <span
              className="inline-flex items-center rounded-full border border-[var(--sl-color-accent)]/40 bg-[var(--sl-color-accent-low)] px-2 py-0.5 text-xs font-semibold text-[var(--sl-color-accent-high)]"
              key={target}
            >
              <code>{target}</code>
            </span>
          ))}
          {convertsTo.length === 0 && !passThrough ? (
            <span className="text-xs text-[var(--sl-color-gray-3)]">
              no conversion implemented
            </span>
          ) : null}
        </div>
      </div>
    </section>
  );
}

export function EventTimeline({ events = [] }) {
  return (
    <ol className="my-6 grid list-none gap-3 p-0">
      {events.map((event, index) => (
        <li
          className="grid grid-cols-[auto_1fr] gap-3"
          key={`${event.name}-${index}`}
        >
          <div className="flex flex-col items-center">
            <span className="grid h-8 w-8 place-items-center rounded-full border border-[var(--sl-color-accent)] bg-[var(--sl-color-accent-low)] text-xs font-bold text-[var(--sl-color-accent-high)]">
              {index + 1}
            </span>
            {index < events.length - 1 ? (
              <span className="h-full w-px bg-[var(--sl-color-gray-5)]" />
            ) : null}
          </div>
          <div className="rounded-xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-3">
            <div className="font-semibold text-[var(--sl-color-white)]">
              <code>{event.name}</code>
            </div>
            {event.description ? (
              <p className="m-0 mt-1 text-sm leading-6 text-[var(--sl-color-gray-2)]">
                {event.description}
              </p>
            ) : null}
          </div>
        </li>
      ))}
    </ol>
  );
}

export function TroubleshootingGrid({ items = [], labels = {} }) {
  const text = {
    causes: "Likely causes",
    next: "Next checks",
    ...labels,
  };

  return (
    <div className="my-6 grid grid-cols-1 gap-4 lg:grid-cols-2">
      {items.map((item) => (
        <section
          className="rounded-2xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-4"
          key={item.symptom}
        >
          <div className="mb-3 flex items-center gap-2 text-[var(--sl-color-white)]">
            <ProxaiIcon name={item.icon ?? "alert"} />
            <strong>{item.symptom}</strong>
          </div>
          {item.causes?.length ? (
            <>
              <div className="text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
                {text.causes}
              </div>
              <ul className="mt-2 list-disc ps-5 text-sm leading-6 text-[var(--sl-color-gray-2)]">
                {item.causes.map((cause) => (
                  <li key={cause}>{cause}</li>
                ))}
              </ul>
            </>
          ) : null}
          {item.next?.length ? (
            <>
              <div className="mt-3 text-xs font-semibold uppercase tracking-wide text-[var(--sl-color-gray-3)]">
                {text.next}
              </div>
              <ul className="mt-2 list-disc ps-5 text-sm leading-6 text-[var(--sl-color-gray-2)]">
                {item.next.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ul>
            </>
          ) : null}
        </section>
      ))}
    </div>
  );
}
