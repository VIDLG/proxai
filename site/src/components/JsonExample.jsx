import { JsonView, allExpanded, defaultStyles } from "react-json-view-lite";

export function JsonExample({ data, expand = true }) {
  return (
    <div className="my-4 overflow-hidden rounded-xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)] p-3">
      <JsonView
        data={data}
        shouldExpandNode={expand ? allExpanded : undefined}
        style={defaultStyles}
      />
    </div>
  );
}
