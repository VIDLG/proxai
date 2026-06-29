import { InlineMarkdown } from "./shared.jsx";

const Detail = ({ label, value }) => {
  if (!value || (Array.isArray(value) && value.length === 0)) return null;
  return (
    <div className="mt-3">
      <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
        {label}
      </div>
      {Array.isArray(value) ? (
        <ul className="mt-1 list-disc ps-5 text-sm leading-6 text-gray-600 dark:text-gray-300">
          {value.map((item) => (
            <li key={item}>
              <InlineMarkdown>{item}</InlineMarkdown>
            </li>
          ))}
        </ul>
      ) : (
        <p className="m-0 mt-1 text-sm leading-6 text-gray-600 dark:text-gray-300">
          <InlineMarkdown>{value}</InlineMarkdown>
        </p>
      )}
    </div>
  );
};

export function ContractCards({
  contracts = [],
  layout = "grid",
  labels = {},
}) {
  const layoutClass =
    layout === "single" ? "grid-cols-1" : "grid-cols-1 lg:grid-cols-2";
  const text = {
    promise: "Promise",
    sourceOwner: "Source owner",
    failureMode: "Failure mode",
    suggestedTests: "Suggested tests",
    relatedDocs: "Related docs",
    ...labels,
  };

  return (
    <div className={`not-prose my-5 grid gap-3 ${layoutClass}`}>
      {contracts.map((contract) => (
        <article
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={contract.id}
        >
          <div className="flex items-start gap-3">
            <span className="inline-flex shrink-0 rounded-full border border-indigo-500/40 bg-indigo-100 px-2 py-0.5 text-xs font-extrabold text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
              {contract.id}
            </span>
            <div className="min-w-0 flex-1">
              <h3 className="m-0 text-base font-semibold text-gray-900 dark:text-white">
                {contract.title}
              </h3>
              <p className="m-0 mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
                <InlineMarkdown>
                  {contract.promise ?? contract.body}
                </InlineMarkdown>
              </p>
              <Detail label={text.sourceOwner} value={contract.sourceOwner} />
              <Detail label={text.failureMode} value={contract.failureMode} />
              <Detail
                label={text.suggestedTests}
                value={contract.suggestedTests}
              />
              {contract.relatedDocs?.length ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {contract.relatedDocs.map((doc) => (
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
            </div>
          </div>
        </article>
      ))}
    </div>
  );
}
