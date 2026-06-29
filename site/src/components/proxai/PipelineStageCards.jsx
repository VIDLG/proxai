import { renderCell, InlineMarkdown } from "./shared.jsx";
export function PipelineStageCards({ stages = [], labels = {} }) {
  const text = {
    modules: "Modules",
    responsibility: "Responsibility",
    ...labels,
  };

  return (
    <ol className="not-prose my-6 grid list-none gap-3 p-0">
      {stages.map((stage, index) => (
        <li
          className="grid grid-cols-[auto_1fr] gap-3 rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={stage.name}
        >
          <span className="grid h-8 w-8 place-items-center rounded-full bg-indigo-600 text-sm font-extrabold text-white dark:bg-indigo-400 dark:text-black">
            {index + 1}
          </span>
          <div>
            <div className="flex flex-wrap items-center gap-2 text-gray-900 dark:text-white">
              <strong>
                {renderCell(stage.name, { code: stage.codeName ?? true })}
              </strong>
            </div>
            <div className="mt-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
              {text.modules}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              {stage.modules?.map((module) => (
                <code
                  className="rounded-md border border-gray-200 bg-gray-900 px-2 py-1 text-xs dark:border-gray-800 dark:bg-black"
                  key={module}
                >
                  {module}
                </code>
              ))}
            </div>
            <div className="mt-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
              {text.responsibility}
            </div>
            <p className="m-0 mt-1 text-sm leading-6 text-gray-600 dark:text-gray-300">
              <InlineMarkdown>{stage.responsibility}</InlineMarkdown>
            </p>
          </div>
        </li>
      ))}
    </ol>
  );
}
