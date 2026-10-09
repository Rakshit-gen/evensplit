import type { ReactNode } from "react";
import type { Expense, Trip } from "./api";
import { decimals, formatMinor, parseAmount, splitAmong } from "./ledger";

interface Props {
  trip: Trip;
  editing: number | null;
  /** The form shown in place of the row being edited. */
  form: ReactNode;
  onEdit: (i: number) => void;
  onDelete: (i: number) => void;
  onDeletePayment: (i: number) => void;
}

function shown(amount: string, currency: string): string {
  const m = parseAmount(amount, decimals(currency));
  return m === null ? amount : formatMinor(m, decimals(currency), currency);
}

function splitText(e: Expense, people: string[]): string {
  const s = e.split;
  if ("equal" in s) {
    if (!s.equal.length) return "split by everyone";
    if (s.equal.length === 1) return `all for ${s.equal[0]}`;
    return `split by ${splitAmong(s, people).join(", ")}`;
  }
  if ("shares" in s) {
    return (
      "by shares: " +
      Object.entries(s.shares)
        .map(([p, n]) => `${p} ${n}`)
        .join(", ")
    );
  }
  return (
    "exact: " +
    Object.entries(s.exact)
      .map(([p, a]) => `${p} ${a}`)
      .join(", ")
  );
}

/** Every expense and payment so far, newest at the bottom like a ledger. */
export default function Entries({ trip, editing, form, onEdit, onDelete, onDeletePayment }: Props) {
  if (!trip.expenses.length && !trip.payments.length) {
    return <p className="muted">Nothing yet. Expenses you add show up here, with the newest at the bottom.</p>;
  }
  return (
    <ol className="ledger">
      {trip.expenses.map((e, i) =>
        editing === i ? (
          <li key={i} className="editing">
            {form}
          </li>
        ) : (
          <li key={i}>
            <button className="entry" onClick={() => onEdit(i)} title="Edit this expense">
              <span className="what">{e.what || "Untitled expense"}</span>
              <span className="meta">
                {e.paid_by} paid, {splitText(e, trip.people)}
              </span>
              <span className="money">
                {shown(e.amount, e.currency ?? trip.currency)}
                {e.currency && e.currency !== trip.currency && <span className="cur"> {e.currency}</span>}
              </span>
            </button>
            <button
              className="quiet delete"
              onClick={() => onDelete(i)}
              aria-label={`Delete ${e.what || "this expense"}`}
            >
              Delete
            </button>
          </li>
        ),
      )}
      {trip.payments.map((p, i) => (
        <li key={`p${i}`} className="payment">
          <div className="entry">
            <span className="what">
              {p.from} paid {p.to}
            </span>
            <span className="meta">settling up</span>
            <span className="money">{shown(p.amount, trip.currency)}</span>
          </div>
          <button
            className="quiet delete"
            onClick={() => onDeletePayment(i)}
            aria-label={`Delete payment from ${p.from} to ${p.to}`}
          >
            Delete
          </button>
        </li>
      ))}
    </ol>
  );
}
