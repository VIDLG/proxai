import { StatusPill } from "./StatusPill.jsx";
export function ProtocolMatrix({ rows = [], labels }) {
  const text = {
    inbound: "Inbound protocol",
    provider: "Provider protocol",
    status: "Status",
    ...labels,
  };

  return (
    <div className="my-4 overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-gray-50 dark:bg-gray-900">
          <tr>
            <th className="border-b border-gray-200 dark:border-gray-800 px-4 py-3 text-left">
              {text.inbound}
            </th>
            <th className="border-b border-gray-200 dark:border-gray-800 px-4 py-3 text-left">
              {text.provider}
            </th>
            <th className="border-b border-gray-200 dark:border-gray-800 px-4 py-3 text-left">
              {text.status}
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              className="border-b border-gray-100 dark:border-gray-800 last:border-0"
              key={`${row.inbound}-${row.provider}-${row.status}`}
            >
              <td className="px-4 py-3">
                <code>{row.inbound}</code>
              </td>
              <td className="px-4 py-3">
                <code>{row.provider}</code>
              </td>
              <td className="px-4 py-3">
                <StatusPill status={row.status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
