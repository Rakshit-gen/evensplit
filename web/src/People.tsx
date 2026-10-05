import { useState } from "react";
import type { Trip } from "./api";
import { isInvolved, renamePerson } from "./ledger";

interface Props {
  trip: Trip;
  save: (t: Trip) => Promise<boolean>;
}

/** Who is on the trip. Click a name to rename or remove them. */
export default function People({ trip, save }: Props) {
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");

  const add = async (e: React.FormEvent) => {
    e.preventDefault();
    const n = name.trim();
    if (!n) return;
    if (await save({ ...trip, people: [...trip.people, n] })) setName("");
  };

  const rename = async (e: React.FormEvent) => {
    e.preventDefault();
    const n = draft.trim();
    if (editing && n && n !== editing && !(await save(renamePerson(trip, editing, n)))) return;
    setEditing(null);
  };

  const remove = async (who: string) => {
    if (await save({ ...trip, people: trip.people.filter((p) => p !== who) })) setEditing(null);
  };

  return (
    <section className="people" aria-labelledby="people-h">
      <h2 id="people-h">People</h2>
      <ul className="names">
        {trip.people.map((p) =>
          editing === p ? (
            <li key={p}>
              <form className="rename" onSubmit={rename}>
                <label className="sr" htmlFor="rename">
                  New name for {p}
                </label>
                <input
                  id="rename"
                  value={draft}
                  autoFocus
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => e.key === "Escape" && setEditing(null)}
                />
                <button type="submit">Rename</button>
                {isInvolved(trip, p) ? (
                  <span className="muted small">On an expense, so can't be removed</span>
                ) : (
                  <button type="button" className="quiet" onClick={() => remove(p)}>
                    Remove
                  </button>
                )}
              </form>
            </li>
          ) : (
            <li key={p}>
              <button
                className="name"
                title={`Rename or remove ${p}`}
                onClick={() => {
                  setEditing(p);
                  setDraft(p);
                }}
              >
                {p}
              </button>
            </li>
          ),
        )}
        <li>
          <form className="add-person" onSubmit={add}>
            <label className="sr" htmlFor="new-person">
              Add a person
            </label>
            <input
              id="new-person"
              placeholder={trip.people.length ? "Add someone" : "Add the first person"}
              value={name}
              autoFocus={trip.people.length < 2}
              onChange={(e) => setName(e.target.value)}
            />
            <button type="submit" disabled={!name.trim()}>
              Add
            </button>
          </form>
        </li>
      </ul>
    </section>
  );
}
