export function PipelineStrip({ steps = [] }) {
  return (
    <ol className="my-5 grid list-none gap-3 p-0">
      {steps.map((step, index) => (
        <li
          className="flex items-start gap-3 rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-3"
          key={step.title}
        >
          <span className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-indigo-600 dark:bg-indigo-400 text-sm font-extrabold text-white dark:text-black">
            {index + 1}
          </span>
          <span>
            <strong className="text-gray-900 dark:text-white">
              {step.title}
            </strong>
            {step.description ? (
              <small className="mt-1 block text-gray-600 dark:text-gray-300">
                {step.description}
              </small>
            ) : null}
          </span>
        </li>
      ))}
    </ol>
  );
}
