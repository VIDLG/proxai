export function StatusPill({ status }) {
  const normalized = String(status ?? "").toLowerCase();
  const supported =
    normalized.includes("pass") ||
    normalized.includes("conversion") ||
    normalized.includes("转换") ||
    normalized.includes("透传");
  const classes = supported
    ? "border-green-500/40 bg-green-100/60 dark:bg-green-950/60 text-green-600 dark:text-green-400"
    : "border-orange-500/40 bg-orange-100/60 dark:bg-orange-950/60 text-orange-600 dark:text-orange-400";

  return (
    <span
      className={`inline-flex rounded-full border px-2 py-0.5 text-xs font-semibold ${classes}`}
    >
      {status}
    </span>
  );
}
