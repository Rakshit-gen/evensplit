import { useCallback, useEffect, useRef, useState } from "react";
import { getTrip, putTrip, type Expense, type Trip, type View } from "./api";
import ExpenseForm from "./ExpenseForm";
import Entries from "./Entries";
import { blankExpense, markPaid, removeExpense, removePayment, restore, type Removed } from "./ledger";
import People from "./People";
import Settings from "./Settings";
import Settle from "./Settle";
import Summary from "./Summary";

export default function App() {
  const [view, setView] = useState<View | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The last payer and currency carry over to the next expense, because
  // receipts tend to come in runs from the same person.
  const [last, setLast] = useState<{ payer: string | null; currency?: string }>({ payer: null });
  const [added, setAdded] = useState(0);
  const [editing, setEditing] = useState<number | null>(null);
  const [removed, setRemoved] = useState<Removed | null>(null);

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

  // Ctrl+Z or Cmd+Z brings back the last deletion, unless the cursor is in
  // a text box where it should undo typing instead.
  const undoRef = useRef<() => void>(() => {});
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey && !t.closest("input, select, textarea")) {
        e.preventDefault();
        undoRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!view) {
    return (
      <main className="page">
        {error ? <p className="error">{error}</p> : <p className="muted">Opening the trip…</p>}
      </main>
    );
  }
  const { trip } = view;
  const blank = { ...blankExpense(trip, last.payer), currency: last.currency };

  const add = async (e: Expense) => {
    if (!(await save({ ...trip, expenses: [...trip.expenses, e] }))) return false;
    setLast({ payer: e.paid_by, currency: e.currency });
    setAdded((n) => n + 1);
    return true;
  };

  const update = async (e: Expense) => {
    if (editing === null) return false;
    const expenses = trip.expenses.map((old, i) => (i === editing ? e : old));
    if (!(await save({ ...trip, expenses }))) return false;
    setEditing(null);
    return true;
  };

  const remove = async ([next, gone]: [Trip, Removed]) => {
    if (await save(next)) {
      setRemoved(gone);
      setEditing(null);
    }
  };

  const undo = async () => {
    if (removed && (await save(restore(trip, removed)))) setRemoved(null);
  };
  undoRef.current = undo;

  return (
    <main className="page">
      <header className="top">
        <img className="logo" src="/logo.svg" alt="" width="32" height="32" />
        <h1>{trip.name || "Untitled trip"}</h1>
        <span className="currency">{trip.currency}</span>
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
      <Settings trip={trip} save={save} />
      <div className="columns">
        <div className="main">
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
          <section aria-labelledby="ledger-h">
            <h2 id="ledger-h">Expenses</h2>
            <Entries
              trip={trip}
              editing={editing}
              form={
                editing !== null && (
                  <ExpenseForm
                    key={`edit-${editing}`}
                    trip={trip}
                    start={trip.expenses[editing]!}
                    editing
                    focus
                    onSubmit={update}
                    onCancel={() => setEditing(null)}
                  />
                )
              }
              onEdit={setEditing}
              onDelete={(i) => remove(removeExpense(trip, i))}
              onDeletePayment={(i) => remove(removePayment(trip, i))}
            />
          </section>
        </div>
        <aside className="side">
          {view.report && (
            <Settle report={view.report} onPaid={(o) => save(markPaid(trip, o, view.report!.decimals))} />
          )}
          {view.summary && <Summary text={view.summary} />}
        </aside>
      </div>
      {removed && (
        <div className="undo" role="status">
          <span>
            Deleted{" "}
            {removed.kind === "expense"
              ? `"${removed.item.what || "Untitled expense"}"`
              : `payment from ${removed.item.from} to ${removed.item.to}`}
            .
          </span>
          <button onClick={undo}>Undo</button>
          <button className="quiet" onClick={() => setRemoved(null)} aria-label="Dismiss">
            Dismiss
          </button>
        </div>
      )}
    </main>
  );
}
