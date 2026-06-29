import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function TroubleshootingGrid({
  items = [],
  labels = {},
  layout = "grid",
}) {
  const layoutClass =
    layout === "single" ? "grid-cols-1" : "grid-cols-1 lg:grid-cols-2";
  const text = {
    causes: "Likely causes",
    next: "Next checks",
    layer: "Layer",
    capture: "Useful capture",
    docs: "Related docs",
    ...labels,
  };

  return (
    <div className={`my-6 grid gap-4 ${layoutClass}`}>
      {items.map((item) => (
        <section
          className="rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4"
          key={item.symptom}
        >
          <div className="mb-3 flex items-center gap-2 text-gray-900 dark:text-white">
            <ProxaiIcon name={item.icon ?? "alert"} />
            <strong>{item.symptom}</strong>
          </div>
          {item.layer || item.capture ? (
            <div className="mb-3 flex flex-wrap gap-2 text-xs">
              {item.layer ? (
                <span className="rounded-full border border-indigo-500/40 bg-indigo-100 px-2 py-0.5 font-semibold text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
                  {text.layer}: {item.layer}
                </span>
              ) : null}
              {item.capture ? (
                <span className="rounded-full border border-gray-300 bg-white px-2 py-0.5 font-semibold text-gray-700 dark:border-gray-700 dark:bg-black dark:text-gray-300">
                  {text.capture}: <code>{item.capture}</code>
                </span>
              ) : null}
            </div>
          ) : null}
          {item.causes?.length ? (
            <>
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                {text.causes}
              </div>
              <ul className="mt-2 list-disc ps-5 text-sm leading-6 text-gray-600 dark:text-gray-300">
                {item.causes.map((cause) => (
                  <li key={cause}>{cause}</li>
                ))}
              </ul>
            </>
          ) : null}
          {item.next?.length ? (
            <>
              <div className="mt-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                {text.next}
              </div>
              <ul className="mt-2 list-disc ps-5 text-sm leading-6 text-gray-600 dark:text-gray-300">
                {item.next.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ul>
            </>
          ) : null}
          {item.docs?.length ? (
            <>
              <div className="mt-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                {text.docs}
              </div>
              <div className="mt-2 flex flex-wrap gap-2">
                {item.docs.map((doc) => (
                  <a
                    className="rounded-full border border-gray-200 bg-white px-2 py-0.5 text-xs font-semibold text-indigo-700 no-underline dark:border-gray-700 dark:bg-black dark:text-indigo-300"
                    href={doc.href}
                    key={doc.href}
                  >
                    {doc.label}
                  </a>
                ))}
              </div>
            </>
          ) : null}
        </section>
      ))}
    </div>
  );
}
