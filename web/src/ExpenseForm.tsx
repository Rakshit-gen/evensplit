import { useState } from "react";
import type { Expense, Split, Trip } from "./api";
import { decimals, exactLeft, formatMinor, parseAmount } from "./ledger";

type Mode = "everyone" | "some" | "shares" | "exact";

interface Props {
  trip: Trip;
  /** The expense being edited, or a blank one to add. */
  start: Expense;
  editing: boolean;
  /** Put the cursor in the first box when the form appears. */
  focus: boolean;
  onSubmit: (e: Expense) => Promise<boolean>;
  onCancel: () => void;
}

function modeOf(s: Split): Mode {
  if ("equal" in s) return s.equal.length ? "some" : "everyone";
  return "shares" in s ? "shares" : "exact";
}

const MODES: [Mode, string][] = [
  ["everyone", "Everyone"],
  ["some", "Some people"],
  ["shares", "Shares"],
  ["exact", "Exact amounts"],
];

/**
 * Add or edit one expense. Enter saves from any field and Escape leaves
 * an edit.
 */
// The parent gives each form a fresh key for each expense, so the fields
// start from `start` and never need resetting by hand.
export default function ExpenseForm({ trip, start, editing, focus, onSubmit, onCancel }: Props) {
  const [what, setWhat] = useState(start.what);
  const [amount, setAmount] = useState(start.amount);
  const [currency, setCurrency] = useState(start.currency ?? trip.currency);
  const [payer, setPayer] = useState(start.paid_by);
  const [mode, setMode] = useState<Mode>(modeOf(start.split));
  const [some, setSome] = useState<string[]>("equal" in start.split ? start.split.equal : []);
  const [shares, setShares] = useState<Record<string, string>>(
    "shares" in start.split ? Object.fromEntries(Object.entries(start.split.shares).map(([k, v]) => [k, String(v)])) : {},
  );
  const [exact, setExact] = useState<Record<string, string>>("exact" in start.split ? start.split.exact : {});
  const [problem, setProblem] = useState<string | null>(null);

  const places = decimals(currency);
  const currencies = [trip.currency, ...Object.keys(trip.rates ?? {}).filter((c) => c !== trip.currency)];
  const left = mode === "exact" ? exactLeft(amount, exact, places) : null;

  const build = (): Expense | string => {
    const total = parseAmount(amount, places);
    if (total === null || total === 0) {
      return places
        ? `Enter the amount as a number above zero, like 1240.50.`
        : `Enter the amount as a whole number above zero; ${currency} has no decimals.`;
    }
    if (!trip.people.includes(payer)) return "Pick who paid.";
    let split: Split;
    if (mode === "everyone") split = { equal: [] };
    else if (mode === "some") {
      if (!some.length) return "Tick at least one person to split between.";
      split = { equal: trip.people.filter((p) => some.includes(p)) };
    } else if (mode === "shares") {
      const out: Record<string, number> = {};
      for (const p of trip.people) {
        const t = (shares[p] ?? "").trim();
        if (!t) continue;
        if (!/^\d+$/.test(t)) return `Shares are whole numbers; ${p} has "${t}".`;
        if (Number(t) > 0) out[p] = Number(t);
      }
      if (!Object.keys(out).length) return "Give at least one person a share.";
      split = { shares: out };
    } else {
      if (left === null) return "One of the exact amounts isn't a number.";
      if (left !== 0) {
        const s = formatMinor(Math.abs(left), places);
        return left > 0 ? `${s} ${currency} still to assign.` : `The parts are ${s} ${currency} over the total.`;
      }
      const out: Record<string, string> = {};
      for (const p of trip.people) if ((exact[p] ?? "").trim()) out[p] = exact[p]!.trim();
      split = { exact: out };
    }
    const e: Expense = { what: what.trim(), paid_by: payer, amount: amount.trim(), split };
    if (currency !== trip.currency) e.currency = currency;
    return e;
  };

  const submit = async (ev: React.FormEvent) => {
    ev.preventDefault();
    const e = build();
    if (typeof e === "string") {
      setProblem(e);
      return;
    }
    setProblem(null);
    await onSubmit(e);
  };

  if (!trip.people.length) {
    return <p className="muted">Add the people on the trip first, then their expenses.</p>;
  }

  return (
    <form
      className="expense-form"
      onSubmit={submit}
      onKeyDown={(e) => {
        if (e.key === "Escape" && editing) onCancel();
      }}
      aria-label={editing ? "Edit expense" : "Add an expense"}
    >
      <div className="row">
        <label className="grow">
          <span>What for</span>
          <input autoFocus={focus} value={what} placeholder="Dinner" onChange={(e) => setWhat(e.target.value)} />
        </label>
        <label className="amount">
          <span>Amount</span>
          <input
            value={amount}
            inputMode="decimal"
            placeholder={places ? "0.00" : "0"}
            onChange={(e) => setAmount(e.target.value)}
            aria-invalid={amount !== "" && parseAmount(amount, places) === null}
          />
        </label>
        {currencies.length > 1 && (
          <label>
            <span>Currency</span>
            <select value={currency} onChange={(e) => setCurrency(e.target.value)}>
              {currencies.map((c) => (
                <option key={c}>{c}</option>
              ))}
            </select>
          </label>
        )}
        <label>
          <span>Paid by</span>
          <select value={payer} onChange={(e) => setPayer(e.target.value)}>
            {trip.people.map((p) => (
              <option key={p}>{p}</option>
            ))}
          </select>
        </label>
      </div>

      <fieldset className="modes">
        <legend>Split between</legend>
        {MODES.map(([m, label]) => (
          <label key={m} className={mode === m ? "on" : ""}>
            <input type="radio" name="mode" checked={mode === m} onChange={() => setMode(m)} />
            {label}
          </label>
        ))}
      </fieldset>

      {mode === "some" && (
        <div className="who">
          {trip.people.map((p) => (
            <label key={p} className="tick">
              <input
                type="checkbox"
                checked={some.includes(p)}
                onChange={(e) => setSome(e.target.checked ? [...some, p] : some.filter((x) => x !== p))}
              />
              {p}
            </label>
          ))}
        </div>
      )}
      {(mode === "shares" || mode === "exact") && (
        <div className="who parts">
          {trip.people.map((p) => (
            <label key={p}>
              <span>{p}</span>
              <input
                inputMode={mode === "shares" ? "numeric" : "decimal"}
                placeholder="0"
                value={(mode === "shares" ? shares : exact)[p] ?? ""}
                onChange={(e) =>
                  mode === "shares"
                    ? setShares({ ...shares, [p]: e.target.value })
                    : setExact({ ...exact, [p]: e.target.value })
                }
              />
            </label>
          ))}
          {mode === "exact" && left !== null && (
            <p className={left === 0 ? "muted small" : "small warn"}>
              {left === 0
                ? "Adds up."
                : left > 0
                  ? `${formatMinor(left, places)} left to assign`
                  : `${formatMinor(-left, places)} over the total`}
            </p>
          )}
        </div>
      )}

      <div className="actions">
        <button type="submit" className="primary">
          {editing ? "Save changes" : "Add expense"}
        </button>
        {editing && (
          <button type="button" className="quiet" onClick={onCancel}>
            Cancel
          </button>
        )}
        {problem && (
          <p className="error small" role="alert">
            {problem}
          </p>
        )}
      </div>
    </form>
  );
}
