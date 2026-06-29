import { renderCell } from "./shared.jsx";
export function ReferenceTable({
  columns = ["Field", "Meaning"],
  rows = [],
  codeFirstColumn = true,
}) {
  return (
    <div className="not-prose my-4 overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-gray-50 dark:bg-gray-900">
          <tr>
            {columns.map((column) => (
              <th
                className="border-b border-gray-200 px-4 py-3 text-left font-semibold text-gray-900 dark:border-gray-800 dark:text-white"
                key={column}
              >
                {column}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => {
            const cells = Array.isArray(row)
              ? row
              : [
                  row.term ?? row.field ?? row.value ?? row.name,
                  row.description ?? row.meaning ?? row.purpose,
                ];
            return (
              <tr
                className="border-b border-gray-100 last:border-0 dark:border-gray-800"
                key={`${cells[0]}-${index}`}
              >
                <td className="whitespace-nowrap px-4 py-3 align-top font-medium text-gray-900 dark:text-white">
                  {renderCell(cells[0], { code: codeFirstColumn })}
                </td>
                {cells.slice(1).map((cell, cellIndex) => (
                  <td
                    className="px-4 py-3 align-top leading-6 text-gray-600 dark:text-gray-300"
                    key={cellIndex}
                  >
                    {renderCell(cell)}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
