import { useState } from "react";
import type { Trip } from "./api";

interface Props {
  trip: Trip;
  save: (t: Trip) => Promise<boolean>;
}

/** Trip name, its currency, and exchange rates for spending in others. */
export default function Settings({ trip, save }: Props) {
  const [name, setName] = useState(trip.name);
  const [currency, setCurrency] = useState(trip.currency);
  const [rates, setRates] = useState<[string, string][]>(Object.entries(trip.rates ?? {}));
  const [saved, setSaved] = useState(false);
  // Expenses without a currency and every payment are in the trip's
  // currency, so changing it later would quietly change what they mean.
  const locked = trip.expenses.length > 0 || trip.payments.length > 0;

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const kept = rates
      .map(([c, r]) => [c.trim().toUpperCase(), r.trim()] as [string, string])
      .filter(([c, r]) => c || r);
    const next = {
      ...trip,
      name: name.trim(),
      currency: currency.trim().toUpperCase(),
      rates: Object.fromEntries(kept),
    };
    setSaved(await save(next));
  };

  const set = (i: number, j: 0 | 1, v: string) => {
    setSaved(false);
    setRates(rates.map((r, k) => (k === i ? ((j === 0 ? [v, r[1]] : [r[0], v]) as [string, string]) : r)));
  };

  return (
    <details className="settings">
      <summary>Trip name, currency and exchange rates</summary>
      <form onSubmit={submit} onChange={() => setSaved(false)}>
        <div className="row">
          <label className="grow">
            <span>Trip name</span>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label className="code">
            <span>Currency</span>
            <input
              value={currency}
              maxLength={3}
              disabled={locked}
              title={locked ? "Fixed once there are expenses, so their amounts keep their meaning." : undefined}
              onChange={(e) => setCurrency(e.target.value)}
            />
          </label>
        </div>
        <p className="muted small">
          Spent in another currency? Add it with what 1 unit is worth in {trip.currency}. The rate is used for every
          expense in that currency.
        </p>
        {rates.map(([c, r], i) => (
          <div className="row rate" key={i}>
            <label className="code">
              <span>Currency</span>
              <input value={c} maxLength={3} placeholder="EUR" onChange={(e) => set(i, 0, e.target.value)} />
            </label>
            <label className="grow">
              <span>1 {c || "unit"} is worth</span>
              <input value={r} inputMode="decimal" placeholder="90.25" onChange={(e) => set(i, 1, e.target.value)} />
            </label>
            <button type="button" className="quiet" onClick={() => setRates(rates.filter((_, k) => k !== i))}>
              Remove
            </button>
          </div>
        ))}
        <div className="actions">
          <button type="button" className="quiet" onClick={() => setRates([...rates, ["", ""]])}>
            Add a currency
          </button>
          <button type="submit">Save trip details</button>
          {saved && <span className="muted small">Saved.</span>}
        </div>
      </form>
    </details>
  );
}
