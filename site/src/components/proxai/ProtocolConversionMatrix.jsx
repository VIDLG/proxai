export function ProtocolConversionMatrix({ labels = {} }) {
  const text = {
    inbound: "Inbound \\ Provider",
    responses: "Responses",
    chat: "Chat",
    anthropic: "Anthropic",
    pass: "Pass-through",
    supported: "Supported",
    unsupported: "Unsupported",
    ...labels,
  };
  const protocols = [
    { key: "openai_responses", label: text.responses },
    { key: "openai_chat_completions", label: text.chat },
    { key: "anthropic_messages", label: text.anthropic },
  ];
  const support = {
    openai_responses: {
      openai_responses: "pass",
      openai_chat_completions: "supported",
      anthropic_messages: "supported",
    },
    openai_chat_completions: {
      openai_responses: "unsupported",
      openai_chat_completions: "pass",
      anthropic_messages: "supported",
    },
    anthropic_messages: {
      openai_responses: "supported",
      openai_chat_completions: "unsupported",
      anthropic_messages: "pass",
    },
  };
  const meta = {
    pass: {
      label: text.pass,
      className:
        "border-green-500/40 bg-green-100/70 text-green-700 dark:bg-green-950/70 dark:text-green-300",
    },
    supported: {
      label: text.supported,
      className:
        "border-indigo-500/40 bg-indigo-100/70 text-indigo-700 dark:bg-indigo-950/70 dark:text-indigo-300",
    },
    unsupported: {
      label: text.unsupported,
      className:
        "border-gray-300 bg-gray-100 text-gray-500 dark:border-gray-700 dark:bg-black dark:text-gray-400",
    },
  };

  return (
    <div className="not-prose my-5 overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
      <table className="m-0 w-full border-collapse text-sm">
        <thead className="bg-gray-50 dark:bg-gray-900">
          <tr>
            <th className="border-b border-gray-200 px-4 py-3 text-left dark:border-gray-800">
              {text.inbound}
            </th>
            {protocols.map((protocol) => (
              <th
                className="border-b border-gray-200 px-4 py-3 text-left dark:border-gray-800"
                key={protocol.key}
              >
                {protocol.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {protocols.map((inbound) => (
            <tr
              className="border-b border-gray-100 last:border-0 dark:border-gray-800"
              key={inbound.key}
            >
              <td className="px-4 py-3 align-top font-semibold text-gray-900 dark:text-white">
                <code>{inbound.key}</code>
              </td>
              {protocols.map((provider) => {
                const state = support[inbound.key][provider.key];
                const item = meta[state];
                return (
                  <td className="px-4 py-3 align-top" key={provider.key}>
                    <span
                      className={`inline-flex rounded-full border px-2 py-0.5 text-xs font-semibold ${item.className}`}
                    >
                      {item.label}
                    </span>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
