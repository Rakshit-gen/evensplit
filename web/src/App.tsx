import { useCallback, useEffect, useState } from "react";
import { getTrip, putTrip, type Expense, type Trip, type View } from "./api";
import ExpenseForm from "./ExpenseForm";
import { blankExpense } from "./ledger";
import People from "./People";

export default function App() {
  const [view, setView] = useState<View | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The last payer and currency carry over to the next expense, because
  // receipts tend to come in runs from the same person.
  const [last, setLast] = useState<{ payer: string | null; currency?: string }>({ payer: null });
  const [added, setAdded] = useState(0);

  useEffect(() => {
    getTrip().then(setView, (e: Error) => setError(e.message));
  }, []);

  // Every change goes to the server, which checks it and saves the file.
  // The page only shows what was saved, so it never disagrees with disk.
  const save = useCallback(async (next: Trip): Promise<boolean> => {
    try {
      setView(await putTrip(next));
      setError(null);
      return true;
    } catch (e) {
      setError((e as Error).message);
      return false;
    }
  }, []);

  if (!view) {
    return <main className="page">{error ? <p className="error">{error}</p> : <p className="muted">Opening the trip…</p>}</main>;
  }
  const { trip } = view;
  const blank = { ...blankExpense(trip, last.payer), currency: last.currency };

  const add = async (e: Expense) => {
    if (!(await save({ ...trip, expenses: [...trip.expenses, e] }))) return false;
    setLast({ payer: e.paid_by, currency: e.currency });
    setAdded((n) => n + 1);
    return true;
  };

  return (
    <main className="page">
      <header className="top">
        <h1>{trip.name || "Untitled trip"}</h1>
      </header>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {view.error && (
        <p className="error" role="alert">
          The trip file has a mistake: {view.error}
        </p>
      )}
      <People trip={trip} save={save} />
      <section aria-labelledby="add-h">
        <h2 id="add-h">Add an expense</h2>
        <ExpenseForm
          key={`new-${added}-${trip.people.join()}`}
          trip={trip}
          start={blank}
          editing={false}
          focus={added > 0 || trip.people.length >= 2}
          onSubmit={add}
          onCancel={() => {}}
        />
      </section>
    </main>
  );
}
