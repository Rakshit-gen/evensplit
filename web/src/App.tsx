import { useCallback, useEffect, useState } from "react";
import { getTrip, putTrip, type Trip, type View } from "./api";
import People from "./People";

export default function App() {
  const [view, setView] = useState<View | null>(null);
  const [error, setError] = useState<string | null>(null);

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
    </main>
  );
}
