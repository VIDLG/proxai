import { icons } from "./shared.jsx";
import { Info } from "lucide-react";
export function ProxaiIcon({ name = "info", size = 18, className = "" }) {
  const Component = icons[name] ?? Info;
  return (
    <Component
      aria-hidden="true"
      className={className}
      size={size}
      strokeWidth={2}
    />
  );
}
