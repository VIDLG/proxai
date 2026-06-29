import { ProxaiIcon } from "./ProxaiIcon.jsx";
export function ProxaiCallout({ type = "note", title, children }) {
  const meta = {
    note: {
      icon: "info",
      label: title ?? "Note",
      accent: "border-s-indigo-500",
    },
    tip: {
      icon: "rocket",
      label: title ?? "Tip",
      accent: "border-s-green-500",
    },
    caution: {
      icon: "alert",
      label: title ?? "Caution",
      accent: "border-s-orange-500",
    },
    success: {
      icon: "check",
      label: title ?? "Success",
      accent: "border-s-green-500",
    },
  }[type] ?? {
    icon: "info",
    label: title ?? "Note",
    accent: "border-s-indigo-500",
  };

  return (
    <aside
      className={`my-5 rounded-xl border border-gray-200 dark:border-gray-800 border-s-4 ${meta.accent} bg-gray-50 dark:bg-gray-900 p-4`}
    >
      <div className="flex items-center gap-2 text-gray-900 dark:text-white">
        <ProxaiIcon name={meta.icon} />
        <strong>{meta.label}</strong>
      </div>
      <div className="mt-2 [&>*:first-child]:mt-0 [&>*:last-child]:mb-0">
        {children}
      </div>
    </aside>
  );
}
