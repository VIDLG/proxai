export function EventTimeline({ events = [] }) {
  return (
    <ol className="my-6 grid list-none gap-3 p-0">
      {events.map((event, index) => (
        <li
          className="grid grid-cols-[auto_1fr] gap-3"
          key={`${event.name}-${index}`}
        >
          <div className="flex flex-col items-center">
            <span className="grid h-8 w-8 place-items-center rounded-full border border-indigo-500 bg-indigo-100 dark:bg-indigo-950 text-xs font-bold text-indigo-700 dark:text-indigo-300">
              {index + 1}
            </span>
            {index < events.length - 1 ? (
              <span className="h-full w-px bg-gray-100 dark:bg-gray-800" />
            ) : null}
          </div>
          <div className="rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-3">
            <div className="font-semibold text-gray-900 dark:text-white">
              <code>{event.name}</code>
            </div>
            {event.description ? (
              <p className="m-0 mt-1 text-sm leading-6 text-gray-600 dark:text-gray-300">
                {event.description}
              </p>
            ) : null}
          </div>
        </li>
      ))}
    </ol>
  );
}
