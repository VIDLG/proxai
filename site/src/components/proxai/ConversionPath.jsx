import { ArrowRight } from "lucide-react";
export function ConversionPath({ from, via, to }) {
  const items = [from, ...(via ?? []), to].filter(Boolean);
  return (
    <div className="my-5 flex flex-col gap-3 rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4 md:flex-row md:items-center">
      {items.map((item, index) => (
        <div
          className="flex flex-1 items-center gap-3"
          key={`${item.title}-${index}`}
        >
          <div className="min-w-0 flex-1 rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-900 dark:bg-black p-3">
            <div className="text-sm font-semibold text-gray-900 dark:text-white">
              {item.title}
            </div>
            {item.description ? (
              <div className="mt-1 text-xs leading-5 text-gray-600 dark:text-gray-300">
                {item.description}
              </div>
            ) : null}
          </div>
          {index < items.length - 1 ? (
            <ArrowRight
              className="hidden shrink-0 text-indigo-700 dark:text-indigo-300 md:block"
              size={20}
            />
          ) : null}
        </div>
      ))}
    </div>
  );
}
