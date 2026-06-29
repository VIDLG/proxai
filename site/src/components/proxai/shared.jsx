import ReactMarkdown from "react-markdown";
import {
  AlertTriangle,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  CircleSlash,
  FileCog,
  Gauge,
  GitBranch,
  KeyRound,
  Laptop,
  Info,
  Network,
  Route,
  Rocket,
  Settings,
  ShieldAlert,
  Shuffle,
  TerminalSquare,
  Timer,
  Zap,
  StopCircle,
} from "lucide-react";

export const icons = {
  alert: AlertTriangle,
  architecture: Network,
  book: BookOpen,
  check: CheckCircle2,
  config: Settings,
  file: FileCog,
  gauge: Gauge,
  flow: GitBranch,
  laptop: Laptop,
  info: Info,
  key: KeyRound,
  protocol: Shuffle,
  route: Route,
  rocket: Rocket,
  security: ShieldAlert,
  terminal: TerminalSquare,
  timer: Timer,
  unsupported: CircleSlash,
  zap: Zap,
  stop: StopCircle,
};

export const InlineMarkdown = ({ children }) => (
  <ReactMarkdown
    allowedElements={["code", "em", "strong", "a", "span", "p"]}
    components={{
      p: ({ children }) => <>{children}</>,
      a: ({ children, href }) => (
        <a
          href={href}
          className="font-medium text-indigo-700 dark:text-indigo-300"
        >
          {children}
        </a>
      ),
    }}
  >
    {String(children ?? "")}
  </ReactMarkdown>
);

export const renderCell = (value, options = {}) => {
  if (value === null || value === undefined) return null;
  if (typeof value !== "string") return value;
  return options.code ? (
    <code>{value}</code>
  ) : (
    <InlineMarkdown>{value}</InlineMarkdown>
  );
};
