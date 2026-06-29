import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function ConfigMap({ groups = [] }) {
  return (
    <div className="my-6 grid grid-cols-1 gap-4 lg:grid-cols-2">
      {groups.map((group) => (
        <section
          className="rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4"
          key={group.title}
        >
          <div className="mb-3 flex items-center gap-2 text-gray-900 dark:text-white">
            <span className="grid h-8 w-8 place-items-center rounded-lg bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300">
              <ProxaiIcon name={group.icon} size={18} />
            </span>
            <strong>{group.title}</strong>
          </div>
          {group.description ? (
            <p className="mb-3 text-sm leading-6 text-gray-600 dark:text-gray-300">
              {group.description}
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            {group.keys?.map((key) => (
              <code
                className="rounded-md border border-gray-200 dark:border-gray-800 bg-gray-900 dark:bg-black px-2 py-1 text-xs"
                key={key}
              >
                {key}
              </code>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
