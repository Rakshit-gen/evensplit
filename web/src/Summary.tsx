import { useState } from "react";

/** The plain-text summary, to paste into the group chat. */
export default function Summary({ text }: { text: string }) {
  const [copied, setCopied] = useState<"yes" | "failed" | null>(null);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied("yes");
    } catch {
      setCopied("failed");
    }
  };

  return (
    <section aria-labelledby="summary-h">
      <h2 id="summary-h">For the group chat</h2>
      <pre className="slip">{text}</pre>
      <div className="actions">
        <button onClick={copy}>Copy summary</button>
        <span className="muted small" role="status">
          {copied === "yes" && "Copied. Paste it into the chat."}
          {copied === "failed" && "The browser blocked copying. Select the text above and copy it instead."}
        </span>
      </div>
    </section>
  );
}
