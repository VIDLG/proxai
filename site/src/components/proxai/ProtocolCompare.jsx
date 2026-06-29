import { renderCell } from "./shared.jsx";
export function ProtocolCompare({ columns = [], rows = [] }) {
  return (
    <div className="my-4 overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-gray-50 dark:bg-gray-900">
          <tr>
            <th className="border-b border-gray-200 dark:border-gray-800 px-4 py-3 text-left">
              {columns[0] ?? "Concept"}
            </th>
            {columns.slice(1).map((col) => (
              <th
                className="border-b border-gray-200 dark:border-gray-800 px-4 py-3 text-left"
                key={col}
              >
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr
              className="border-b border-gray-100 dark:border-gray-800 last:border-0"
              key={`${row[0]}-${index}`}
            >
              <td className="px-4 py-3 align-top font-semibold text-gray-900 dark:text-white">
                {row[0]}
              </td>
              {row.slice(1).map((cell, cellIndex) => (
                <td
                  className="px-4 py-3 align-top text-gray-600 dark:text-gray-300"
                  key={cellIndex}
                >
                  {renderCell(cell)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
