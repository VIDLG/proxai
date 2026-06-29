import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function ModuleGrid({ modules = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
      {modules.map((module) => (
        <section
          className="rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4"
          key={module.name}
        >
          <div className="mb-2 flex items-center gap-2 text-gray-900 dark:text-white">
            <ProxaiIcon name={module.icon ?? "file"} />
            <strong>
              <code>{module.name}</code>
            </strong>
          </div>
          <p className="m-0 text-sm leading-6 text-gray-600 dark:text-gray-300">
            {module.description}
          </p>
        </section>
      ))}
    </div>
  );
}
