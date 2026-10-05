import { expect, test } from "vitest";
import type { Trip } from "./api";
import {
  blankExpense,
  decimals,
  exactLeft,
  formatMinor,
  isInvolved,
  markPaid,
  parseAmount,
  plainAmount,
  removeExpense,
  renamePerson,
  restore,
} from "./ledger";

const trip = (): Trip => ({
  name: "Goa",
  currency: "INR",
  people: ["Asha", "Ben", "Chitra"],
  expenses: [
    { what: "Dinner", paid_by: "Asha", amount: "300", split: { equal: [] } },
    { what: "Taxi", paid_by: "Ben", amount: "90", split: { exact: { Ben: "40", Chitra: "50" } } },
    { what: "Boat", paid_by: "Ben", amount: "60", split: { equal: ["Ben"] } },
  ],
  payments: [],
});

test("reads amounts the way the server does", () => {
  expect(parseAmount("1,240.50", 2)).toBe(124050);
  expect(parseAmount(" 12 ", 2)).toBe(1200);
  expect(parseAmount(".5", 2)).toBe(50);
  expect(parseAmount("1500", 0)).toBe(1500);
  expect(parseAmount("10.005", 2)).toBeNull();
  expect(parseAmount("15.5", 0)).toBeNull();
  expect(parseAmount("", 2)).toBeNull();
  expect(parseAmount(".", 2)).toBeNull();
  expect(parseAmount("-3", 2)).toBeNull();
  expect(parseAmount("1e5", 2)).toBeNull();
  expect(decimals("JPY")).toBe(0);
  expect(decimals("KWD")).toBe(3);
  expect(decimals("INR")).toBe(2);
});

test("formats minor units with grouping", () => {
  expect(formatMinor(124050, 2)).toBe("1,240.50");
  expect(formatMinor(5, 2)).toBe("0.05");
  expect(formatMinor(-123456789, 2)).toBe("-1,234,567.89");
  expect(formatMinor(100000, 0)).toBe("100,000");
  expect(formatMinor(1250, 3)).toBe("1.250");
  expect(plainAmount(124050, 2)).toBe("1240.50");
});

test("a new expense defaults to the last payer and everyone", () => {
  expect(blankExpense(trip(), "Ben")).toEqual({ what: "", paid_by: "Ben", amount: "", split: { equal: [] } });
  expect(blankExpense(trip(), null).paid_by).toBe("Asha");
  expect(blankExpense(trip(), "Someone who left").paid_by).toBe("Asha");
});

test("an exact split shows what is left to hand out", () => {
  expect(exactLeft("100", { Asha: "30", Ben: "" }, 2)).toBe(7000);
  expect(exactLeft("100", { Asha: "30", Ben: "70.00" }, 2)).toBe(0);
  expect(exactLeft("100", { Asha: "130" }, 2)).toBe(-3000);
  expect(exactLeft("", { Asha: "30" }, 2)).toBeNull();
  expect(exactLeft("100", { Asha: "3x" }, 2)).toBeNull();
});

test("undo puts a deleted expense back in its place", () => {
  const t = trip();
  const [without, removed] = removeExpense(t, 1);
  expect(without.expenses.map((e) => e.what)).toEqual(["Dinner", "Boat"]);
  expect(restore(without, removed)).toEqual(t);
  // Still works after something else was deleted in between.
  const [shorter] = removeExpense(without, 1);
  expect(restore(shorter, removed).expenses.map((e) => e.what)).toEqual(["Dinner", "Taxi"]);
});

test("marking a payment done records it in plain form", () => {
  const t = markPaid(trip(), { from: "Chitra", to: "Asha", amount: 124050 }, 2);
  expect(t.payments).toEqual([{ from: "Chitra", to: "Asha", amount: "1240.50" }]);
});

test("renaming someone carries their entries with them", () => {
  const t = renamePerson(markPaid(trip(), { from: "Chitra", to: "Ben", amount: 100 }, 2), "Ben", "Benjamin");
  expect(t.people).toEqual(["Asha", "Benjamin", "Chitra"]);
  expect(t.expenses[1]!.paid_by).toBe("Benjamin");
  expect(t.expenses[1]!.split).toEqual({ exact: { Benjamin: "40", Chitra: "50" } });
  expect(t.expenses[2]!.split).toEqual({ equal: ["Benjamin"] });
  expect(t.payments[0]!.to).toBe("Benjamin");
});

test("people in an everyone split count as involved", () => {
  const t = trip();
  expect(isInvolved(t, "Chitra")).toBe(true);
  const fresh = { ...t, expenses: [t.expenses[2]!], people: [...t.people, "Dev"] };
  expect(isInvolved(fresh, "Dev")).toBe(false);
  expect(isInvolved(fresh, "Ben")).toBe(true);
});
