import { JsonView, allExpanded, defaultStyles } from "react-json-view-lite";

export function JsonExample({ data, expand = true }) {
  return (
    <div className="my-4 overflow-hidden rounded-xl border border-gray-200 bg-gray-50 p-3 dark:border-gray-800 dark:bg-gray-900">
      <JsonView
        data={data}
        shouldExpandNode={expand ? allExpanded : undefined}
        style={defaultStyles}
      />
    </div>
  );
}
