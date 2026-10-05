// Calls to evensplit-web. Shapes mirror src/trip.rs and src/report.rs.

export type Split = { equal: string[] } | { shares: Record<string, number> } | { exact: Record<string, string> };

export interface Expense {
  what: string;
  paid_by: string;
  amount: string;
  currency?: string;
  split: Split;
}

export interface Payment {
  from: string;
  to: string;
  amount: string;
}

export interface Trip {
  name: string;
  currency: string;
  rates?: Record<string, string>;
  people: string[];
  expenses: Expense[];
  payments: Payment[];
}

export interface Balance {
  name: string;
  paid: number;
  share: number;
  settled: number;
  net: number;
}

export interface Owed {
  from: string;
  to: string;
  amount: number;
}

export interface Report {
  name: string;
  currency: string;
  decimals: number;
  total: number;
  balances: Balance[];
  payments: Owed[];
}

// The trip as saved, plus what follows from it. `error` is set instead of
// `report` when the file has a mistake in it.
export interface View {
  trip: Trip;
  report: Report | null;
  summary: string | null;
  error: string | null;
}

async function call(init?: RequestInit): Promise<View> {
  let res: Response;
  try {
    res = await fetch("/api/trip", init);
  } catch {
    throw new Error("Can't reach evensplit-web. Check it is still running, then try again.");
  }
  const body = await res.json().catch(() => null);
  if (!res.ok) {
    throw new Error(body?.error ?? `The server answered ${res.status}. Check evensplit-web is still running.`);
  }
  return body as View;
}

export function getTrip(): Promise<View> {
  return call();
}

export function putTrip(trip: Trip): Promise<View> {
  return call({
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(trip),
  });
}
