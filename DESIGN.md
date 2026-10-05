# Design notes

The page is the green columnar pad people used to keep a shared kitty in: pale green paper, ruled lines, names down
the left, money in a right-hand column, and debts written in red. Everyone recognises that page as "the accounts", and
it puts the numbers first, which is all the tool is for.

## Colour

| Token | Value | Job |
| --- | --- | --- |
| `--paper` | `#f4f7f0` | Page. The faint green of ledger paper. |
| `--sheet` | `#fbfcf8` | Inputs and the summary slip, a shade lighter so they read as places to write. |
| `--rule` | `#c5d6bf` | Ruled lines between entries and around inputs. |
| `--rule-strong` | `#8fae86` | Heading rules and the double rule under totals. |
| `--ink` | `#1c2620` | Text, and the fill of the main button. 14.4:1 on paper. |
| `--muted` | `#4f5f53` | Secondary lines (who paid, how it was split). 6.3:1 on paper. |
| `--owes` | `#a1241b` | Red ink: what someone owes. Used for nothing else. 7.0:1. |
| `--gets` | `#1d5e33` | Green ink: what someone gets back. 7.2:1. |
| `--focus` | `#1f57c3` | Keyboard focus ring, the one blue on the page. |

Red and green always come with a sign or a word ("owes", minus, plus), so the page still reads for people who can't
tell them apart.

## Type

Atkinson Hyperlegible Next for everything, at 400 and 600, loaded from the bundle. It was drawn so that 1, l and I,
or 0 and O, can't be confused, which matters when people copy amounts off the screen into a payment app. Every amount
is set with tabular figures and right-aligned, so a column of money lines up digit under digit.

## Layout

One header with the trip's name, then the people on the trip as a row of names. Below that, two columns on a wide
screen: on the left the form to add an expense and the list of entries; on the right what follows from them (settle
up, balances, the summary for the chat). Under 860 px it is one column in the same order, so on a phone the form is
the first thing under the names.

The signature piece is the ledger list: each entry on its own ruled line, description and who paid on the left, amount
in the money column on the right. Payments that settle a debt sit in the same list in muted ink, because they are
entries too.

Radius is 3 px on inputs and buttons and nothing else; ruled paper has no rounded cards. Spacing steps are 4, 8, 12,
16, 24 and 40 px.

## Motion

Nothing moves except colour on hover and focus, over 120 ms. Saving is fast enough on a local server that there is
nothing to animate. Under `prefers-reduced-motion` even that is off.

## Voice

The page talks like the friend who keeps the kitty: short, plain, in names and amounts. It says "owes" and "gets
back", never "debtor", "creditor" or "balance due". Errors say what is wrong with which entry and what to change.
Sentence case throughout.
