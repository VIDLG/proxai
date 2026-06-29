import { ArrowRight } from "lucide-react";
import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function FeatureGrid({ items = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 md:grid-cols-2">
      {items.map((item) => (
        <a
          className="group grid grid-cols-[auto_1fr_auto] items-start gap-3 rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4 no-underline transition hover:border-indigo-500 hover:bg-gray-100 dark:hover:bg-gray-800"
          href={item.href}
          key={item.href ?? item.title}
        >
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300">
            <ProxaiIcon name={item.icon} size={20} />
          </span>
          <span className="grid gap-1">
            <strong className="text-gray-900 dark:text-white">
              {item.title}
            </strong>
            <span className="text-sm leading-6 text-gray-600 dark:text-gray-300">
              {item.description}
            </span>
          </span>
          <ArrowRight
            className="mt-1 text-gray-500 transition group-hover:translate-x-0.5 group-hover:text-indigo-700 dark:group-hover:text-indigo-300"
            aria-hidden="true"
            size={18}
          />
        </a>
      ))}
    </div>
  );
}
