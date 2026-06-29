import { InlineMarkdown } from "./shared.jsx";
import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function ReasonMap({ groups = [], labels = {}, layout = "grid" }) {
  const layoutClass =
    layout === "single" ? "grid-cols-1" : "grid-cols-1 lg:grid-cols-2";
  const text = {
    values: "Values",
    mapsTo: "Maps to",
    notes: "Notes",
    ...labels,
  };

  return (
    <div className={`not-prose my-6 grid gap-4 ${layoutClass}`}>
      {groups.map((group) => (
        <section
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={group.title}
        >
          <div className="mb-3 flex items-center gap-2 text-gray-900 dark:text-white">
            <span className="grid h-8 w-8 place-items-center rounded-lg bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
              <ProxaiIcon name={group.icon ?? "stop"} size={18} />
            </span>
            <strong>{group.title}</strong>
          </div>
          <div className="space-y-3">
            {group.items?.map((item) => (
              <article
                className="rounded-xl border border-gray-200 bg-white p-3 dark:border-gray-800 dark:bg-black"
                key={item.value}
              >
                <div className="flex flex-wrap items-center gap-2">
                  <code>{item.value}</code>
                  {item.mapsTo ? (
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {text.mapsTo} <code>{item.mapsTo}</code>
                    </span>
                  ) : null}
                </div>
                {item.note ? (
                  <p className="m-0 mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
                    <InlineMarkdown>{item.note}</InlineMarkdown>
                  </p>
                ) : null}
              </article>
            ))}
          </div>
          {group.note ? (
            <p className="m-0 mt-3 text-sm leading-6 text-gray-600 dark:text-gray-300">
              <InlineMarkdown>{group.note}</InlineMarkdown>
            </p>
          ) : null}
        </section>
      ))}
    </div>
  );
}
