import type { Owed, Report } from "./api";
import { formatMinor } from "./ledger";

interface Props {
  report: Report;
  onPaid: (o: Owed) => void;
}

/** Where everyone stands, and the payments that would square it. */
export default function Settle({ report, onPaid }: Props) {
  const m = (v: number) => formatMinor(v, report.decimals);
  return (
    <>
      <section aria-labelledby="settle-h">
        <h2 id="settle-h">Settle up</h2>
        {report.payments.length ? (
          <ul className="payments">
            {report.payments.map((p) => (
              <li key={`${p.from}>${p.to}`}>
                <span>
                  <b>{p.from}</b> pays <b>{p.to}</b>
                </span>
                <span className="money">{m(p.amount)}</span>
                <button onClick={() => onPaid(p)} title="Record this payment as made">
                  Mark paid
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">
            {report.balances.length ? "Everyone is even. Nothing to pay." : "No one on the trip yet."}
          </p>
        )}
      </section>
      {report.balances.length > 0 && (
        <section aria-labelledby="balances-h">
          <h2 id="balances-h">Balances</h2>
          <table className="balances">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col" className="num">
                  Paid
                </th>
                <th scope="col" className="num">
                  Share
                </th>
                <th scope="col" className="num">
                  Stands
                </th>
              </tr>
            </thead>
            <tbody>
              {report.balances.map((b) => (
                <tr key={b.name}>
                  <th scope="row">{b.name}</th>
                  <td className="num money">{m(b.paid)}</td>
                  <td className="num money">{m(b.share)}</td>
                  <td className={`num money ${b.net < 0 ? "owes" : b.net > 0 ? "gets" : ""}`}>
                    {b.net === 0 ? "even" : b.net > 0 ? `+${m(b.net)}` : `−${m(-b.net)}`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">
            Total spent {m(report.total)} {report.currency}. Plus means they get money back, minus means they owe.
          </p>
        </section>
      )}
    </>
  );
}
