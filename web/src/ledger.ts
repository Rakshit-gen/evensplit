// Small pure helpers behind the page: reading what people type into amount
// boxes, showing minor units, and the edits the page makes to a trip.
// The server checks every saved trip again; these just keep typing quick.

import type { Expense, Owed, Split, Trip } from "./api";

// Same table as src/money.rs.
const ZERO = "BIF CLP DJF GNF ISK JPY KMF KRW PYG RWF UGX VND VUV XAF XOF XPF".split(" ");
const THREE = "BHD IQD JOD KWD LYD OMR TND".split(" ");

export function decimals(currency: string): number {
  if (ZERO.includes(currency)) return 0;
  if (THREE.includes(currency)) return 3;
  return 2;
}

/** Minor units from "1,240.50", or null if it isn't an amount. */
export function parseAmount(text: string, places: number): number | null {
  const s = text.replace(/[,\s]/g, "");
  const m = /^(\d*)(?:\.(\d*))?$/.exec(s);
  if (!m || s === "" || s === "." || (m[2] ?? "").length > places) return null;
  const whole = m[1] || "0";
  const frac = (m[2] ?? "").padEnd(places, "0");
  if (whole.length > 15) return null;
  return Number(whole + frac);
}

/** "1,240.50" from minor units. */
export function formatMinor(minor: number, places: number): string {
  const sign = minor < 0 ? "-" : "";
  const digits = String(Math.abs(minor)).padStart(places + 1, "0");
  const whole = digits.slice(0, digits.length - places).replace(/\B(?=(\d{3})+$)/g, ",");
  return places ? `${sign}${whole}.${digits.slice(-places)}` : `${sign}${whole}`;
}

/** Plain "1240.50" for saving: no grouping, all the places. */
export function plainAmount(minor: number, places: number): string {
  return formatMinor(minor, places).replace(/,/g, "");
}

/** Who is in on an expense. */
export function splitAmong(split: Split, people: string[]): string[] {
  if ("equal" in split) return split.equal.length ? split.equal : people;
  const parts = "shares" in split ? split.shares : split.exact;
  return people.filter((p) => p in parts);
}

/** A blank expense: paid by whoever paid last, shared by everyone. */
export function blankExpense(trip: Trip, lastPayer: string | null): Expense {
  const payer = lastPayer && trip.people.includes(lastPayer) ? lastPayer : (trip.people[0] ?? "");
  return { what: "", paid_by: payer, amount: "", split: { equal: [] } };
}

/**
 * For an exact split, how much of the total is still to hand out, in minor
 * units. Null while the total or a part isn't a valid amount yet.
 */
export function exactLeft(amount: string, parts: Record<string, string>, places: number): number | null {
  const total = parseAmount(amount, places);
  if (total === null) return null;
  let left = total;
  for (const text of Object.values(parts)) {
    if (text.trim() === "") continue;
    const p = parseAmount(text, places);
    if (p === null) return null;
    left -= p;
  }
  return left;
}

/** What the page can put back after a delete. */
export type Removed =
  | { kind: "expense"; index: number; item: Expense }
  | { kind: "payment"; index: number; item: Trip["payments"][number] };

export function removeExpense(trip: Trip, index: number): [Trip, Removed] {
  const item = trip.expenses[index]!;
  return [
    { ...trip, expenses: trip.expenses.filter((_, i) => i !== index) },
    { kind: "expense", index, item },
  ];
}

export function removePayment(trip: Trip, index: number): [Trip, Removed] {
  const item = trip.payments[index]!;
  return [
    { ...trip, payments: trip.payments.filter((_, i) => i !== index) },
    { kind: "payment", index, item },
  ];
}

/** Put a deleted entry back where it was, even if the list changed since. */
export function restore(trip: Trip, removed: Removed): Trip {
  if (removed.kind === "expense") {
    const expenses = [...trip.expenses];
    expenses.splice(Math.min(removed.index, expenses.length), 0, removed.item);
    return { ...trip, expenses };
  }
  const payments = [...trip.payments];
  payments.splice(Math.min(removed.index, payments.length), 0, removed.item);
  return { ...trip, payments };
}

/** Record a suggested payment as made. */
export function markPaid(trip: Trip, owed: Owed, places: number): Trip {
  const payment = { from: owed.from, to: owed.to, amount: plainAmount(owed.amount, places) };
  return { ...trip, payments: [...trip.payments, payment] };
}

/** Rename someone everywhere they appear, so their history follows them. */
export function renamePerson(trip: Trip, from: string, to: string): Trip {
  const n = (p: string) => (p === from ? to : p);
  const keys = <T,>(o: Record<string, T>) =>
    Object.fromEntries(Object.entries(o).map(([k, v]) => [n(k), v]));
  const split = (s: Split): Split =>
    "equal" in s ? { equal: s.equal.map(n) } : "shares" in s ? { shares: keys(s.shares) } : { exact: keys(s.exact) };
  return {
    ...trip,
    people: trip.people.map(n),
    expenses: trip.expenses.map((e) => ({ ...e, paid_by: n(e.paid_by), split: split(e.split) })),
    payments: trip.payments.map((p) => ({ ...p, from: n(p.from), to: n(p.to) })),
  };
}

/**
 * Whether anything on the trip mentions someone. Shared-by-everyone counts:
 * taking a person off would quietly change those splits for everyone else.
 */
export function isInvolved(trip: Trip, name: string): boolean {
  return (
    trip.expenses.some((e) => e.paid_by === name || splitAmong(e.split, trip.people).includes(name)) ||
    trip.payments.some((p) => p.from === name || p.to === name)
  );
}
