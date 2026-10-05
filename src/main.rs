use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use evensplit::{Report, Trip, money};

/// Show who owes whom on a trip, and the fewest payments that settle it.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The trip file (JSON).
    trip: PathBuf,
    /// Print balances and payments as JSON, amounts in minor units.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("evensplit: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let text = std::fs::read_to_string(&cli.trip)
        .with_context(|| format!("can't read {}", cli.trip.display()))?;
    let trip: Trip = serde_json::from_str(&text)
        .with_context(|| format!("{} isn't a trip file", cli.trip.display()))?;
    let report = Report::new(&trip).with_context(|| cli.trip.display().to_string())?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", table(&report, trip.expenses.len()));
    }
    Ok(())
}

/// Balances and payments with the amounts lined up on the right.
fn table(r: &Report, expenses: usize) -> String {
    let m = |v: i64| money::format(v, &r.currency);
    let mut out = String::new();
    let noun = if expenses == 1 { "expense" } else { "expenses" };
    if !r.name.trim().is_empty() {
        out += &format!("{}: ", r.name.trim());
    }
    out += &format!("{expenses} {noun}, {} {}\n\n", m(r.total), r.currency);

    let name_w = r
        .balances
        .iter()
        .map(|b| b.name.chars().count())
        .max()
        .unwrap_or(0);
    let amt_w = r
        .balances
        .iter()
        .flat_map(|b| [m(b.paid), m(b.share), m(b.net.abs())])
        .map(|s| s.len())
        .max()
        .unwrap_or(0);
    // Only show settled payments when there are some, to keep the table narrow.
    let settled = r.balances.iter().any(|b| b.settled != 0);
    let head = if settled { "settled" } else { "" };
    let set_w = if settled { amt_w.max(7) } else { 0 };
    let header = format!(
        "{:name_w$}  {:>amt_w$}  {:>amt_w$}  {head:>set_w$}",
        "", "paid", "share"
    );
    out += header.trim_end();
    out += "\n";
    for b in &r.balances {
        let stands = match b.net {
            0 => "even".to_string(),
            n if n > 0 => format!("gets back {}", m(n)),
            n => format!("owes {}", m(-n)),
        };
        let set = if settled { m(b.settled) } else { String::new() };
        out += &format!(
            "{:name_w$}  {:>amt_w$}  {:>amt_w$}  {set:>set_w$}{}{stands}\n",
            b.name,
            m(b.paid),
            m(b.share),
            if settled { "  " } else { "" },
        );
    }
    if r.payments.is_empty() {
        out += "\nEveryone is even.\n";
        return out;
    }
    out += "\nTo settle up:\n";
    let lines: Vec<String> = r
        .payments
        .iter()
        .map(|p| format!("{} pays {}", p.from, p.to))
        .collect();
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    for (l, p) in lines.iter().zip(&r.payments) {
        out += &format!("  {l:w$}  {:>amt_w$}\n", m(p.amount));
    }
    out
}
