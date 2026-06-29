export function ProtocolQuickFacts({
  protocol,
  requestPath,
  codeAreas = [],
  convertsTo = [],
  passThrough = true,
}) {
  return (
    <section className="not-prose my-6 rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4">
      <div className="grid gap-3 md:grid-cols-2">
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
            Protocol value
          </div>
          <div className="mt-1">
            <code className="rounded-md border border-gray-200 dark:border-gray-800 bg-gray-900 dark:bg-black px-2 py-1">
              {protocol}
            </code>
          </div>
        </div>
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
            Request path
          </div>
          <div className="mt-1">
            <code className="rounded-md border border-gray-200 dark:border-gray-800 bg-gray-900 dark:bg-black px-2 py-1">
              {requestPath}
            </code>
          </div>
        </div>
      </div>
      <div className="mt-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
          Main code areas
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          {codeAreas.map((area) => (
            <code
              className="rounded-md border border-gray-200 dark:border-gray-800 bg-gray-900 dark:bg-black px-2 py-1 text-xs"
              key={area}
            >
              {area}
            </code>
          ))}
        </div>
      </div>
      <div className="mt-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
          Conversion targets
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {passThrough ? (
            <span className="inline-flex items-center rounded-full border border-green-500/40 bg-green-100/60 dark:bg-green-950/60 px-2 py-0.5 text-xs font-semibold text-green-600 dark:text-green-400">
              pass-through self
            </span>
          ) : null}
          {convertsTo.map((target) => (
            <span
              className="inline-flex items-center rounded-full border border-indigo-500/40 bg-indigo-100 dark:bg-indigo-950 px-2 py-0.5 text-xs font-semibold text-indigo-700 dark:text-indigo-300"
              key={target}
            >
              <code>{target}</code>
            </span>
          ))}
          {convertsTo.length === 0 && !passThrough ? (
            <span className="text-xs text-gray-500 dark:text-gray-400">
              no conversion implemented
            </span>
          ) : null}
        </div>
      </div>
    </section>
  );
}
