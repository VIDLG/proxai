import { InlineMarkdown } from "./shared.jsx";
export function TaskSteps({ steps = [], labels = {} }) {
  const text = {
    goal: "Goal",
    do: "Do",
    verify: "Verify",
    docs: "Related docs",
    ...labels,
  };

  return (
    <ol className="not-prose my-6 grid list-none gap-4 p-0">
      {steps.map((step, index) => (
        <li
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={`${step.title}-${index}`}
        >
          <div className="flex items-start gap-3">
            <span className="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-indigo-600 text-sm font-extrabold text-white dark:bg-indigo-400 dark:text-black">
              {index + 1}
            </span>
            <div className="min-w-0 flex-1">
              <h3 className="m-0 text-base font-semibold text-gray-900 dark:text-white">
                {step.title}
              </h3>
              {step.goal ? (
                <p className="m-0 mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
                  <strong>{text.goal}:</strong>{" "}
                  <InlineMarkdown>{step.goal}</InlineMarkdown>
                </p>
              ) : null}
              {step.do?.length ? (
                <div className="mt-3">
                  <div className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                    {text.do}
                  </div>
                  <ul className="mt-2 list-disc ps-5 text-sm leading-6 text-gray-600 dark:text-gray-300">
                    {step.do.map((item) => (
                      <li key={item}>
                        <InlineMarkdown>{item}</InlineMarkdown>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
              {step.verify ? (
                <p className="m-0 mt-3 text-sm leading-6 text-gray-600 dark:text-gray-300">
                  <strong>{text.verify}:</strong>{" "}
                  <InlineMarkdown>{step.verify}</InlineMarkdown>
                </p>
              ) : null}
              {step.docs?.length ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {step.docs.map((doc) => (
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
        </li>
      ))}
    </ol>
  );
}
