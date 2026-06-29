import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function DocSectionMap({ sections = [] }) {
  return (
    <div className="not-prose my-6 grid grid-cols-1 gap-4 lg:grid-cols-4">
      {sections.map((section) => (
        <section
          className="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-900"
          key={section.title}
        >
          <div className="mb-3 flex items-center gap-2 text-gray-900 dark:text-white">
            <span className="grid h-8 w-8 place-items-center rounded-lg bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
              <ProxaiIcon name={section.icon ?? "book"} size={18} />
            </span>
            <strong>{section.title}</strong>
          </div>
          <p className="m-0 text-sm leading-6 text-gray-600 dark:text-gray-300">
            {section.description}
          </p>
          {section.audience ? (
            <div className="mt-3 rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-semibold text-gray-600 dark:border-gray-800 dark:bg-black dark:text-gray-300">
              {section.audience}
            </div>
          ) : null}
        </section>
      ))}
    </div>
  );
}
