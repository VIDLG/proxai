import { InlineMarkdown } from "./shared.jsx";

export function GlossaryCards({ terms = [], labels = {}, layout = "grid" }) {
  const layoutClass =
    layout === "single" ? "grid-cols-1" : "grid-cols-1 lg:grid-cols-2";
  const text = {
    definition: "Definition",
    doNotConfuseWith: "Do not confuse with",
    usedIn: "Used in",
    relatedDocs: "Related docs",
    ...labels,
  };

  return (
    <div className={`not-prose my-6 grid gap-4 ${layoutClass}`}>
      {terms.map((term) => (
        <article
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={term.term}
        >
          <h3 className="m-0 text-base font-semibold text-gray-900 dark:text-white">
            <code>{term.term}</code>
          </h3>
          <p className="m-0 mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
            <InlineMarkdown>{term.definition}</InlineMarkdown>
          </p>
          {term.doNotConfuseWith ? (
            <div className="mt-3 rounded-xl border border-orange-200 bg-orange-50 p-3 text-sm leading-6 text-orange-900 dark:border-orange-900 dark:bg-orange-950/40 dark:text-orange-100">
              <strong>{text.doNotConfuseWith}:</strong>{" "}
              <InlineMarkdown>{term.doNotConfuseWith}</InlineMarkdown>
            </div>
          ) : null}
          {term.usedIn?.length ? (
            <div className="mt-3">
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                {text.usedIn}
              </div>
              <div className="mt-2 flex flex-wrap gap-2">
                {term.usedIn.map((item) => (
                  <span
                    className="rounded-full border border-gray-200 bg-white px-2 py-0.5 text-xs font-semibold text-gray-700 dark:border-gray-700 dark:bg-black dark:text-gray-300"
                    key={item}
                  >
                    {item}
                  </span>
                ))}
              </div>
            </div>
          ) : null}
          {term.relatedDocs?.length ? (
            <div className="mt-3 flex flex-wrap gap-2">
              {term.relatedDocs.map((doc) => (
                <a
                  className="rounded-full border border-gray-200 bg-white px-2 py-0.5 text-xs font-semibold text-indigo-700 no-underline dark:border-gray-700 dark:bg-black dark:text-indigo-300"
                  href={doc.href}
                  key={doc.href}
                >
                  {doc.label}
                </a>
              ))}
            </div>
          ) : null}
        </article>
      ))}
    </div>
  );
}
