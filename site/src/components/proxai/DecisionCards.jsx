import { InlineMarkdown } from "./shared.jsx";
import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function DecisionCards({ items = [], labels = {} }) {
  const text = {
    when: "When",
    choose: "Choose",
    why: "Why",
    ...labels,
  };

  return (
    <div className="not-prose my-6 grid gap-4 md:grid-cols-2">
      {items.map((item) => (
        <article
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={item.title}
        >
          <div className="mb-3 flex items-center gap-2 text-gray-900 dark:text-white">
            <span className="grid h-8 w-8 place-items-center rounded-lg bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
              <ProxaiIcon name={item.icon ?? "route"} size={18} />
            </span>
            <strong>{item.title}</strong>
          </div>
          <div className="space-y-3 text-sm leading-6 text-gray-600 dark:text-gray-300">
            {item.when ? (
              <p className="m-0">
                <strong className="text-gray-900 dark:text-white">
                  {text.when}:
                </strong>{" "}
                <InlineMarkdown>{item.when}</InlineMarkdown>
              </p>
            ) : null}
            {item.choose ? (
              <p className="m-0">
                <strong className="text-gray-900 dark:text-white">
                  {text.choose}:
                </strong>{" "}
                <InlineMarkdown>{item.choose}</InlineMarkdown>
              </p>
            ) : null}
            {item.why ? (
              <p className="m-0">
                <strong className="text-gray-900 dark:text-white">
                  {text.why}:
                </strong>{" "}
                <InlineMarkdown>{item.why}</InlineMarkdown>
              </p>
            ) : null}
          </div>
        </article>
      ))}
    </div>
  );
}
