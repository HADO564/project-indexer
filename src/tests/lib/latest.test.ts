import { describe, expect, it } from "vitest";
import { latestOnly } from "$lib/latest";

describe("latestOnly", () => {
  it("keeps a ticket current until a newer one is issued", () => {
    const ticket = latestOnly();
    const first = ticket();
    expect(first()).toBe(true);
    const second = ticket();
    expect(first()).toBe(false);
    expect(second()).toBe(true);
  });

  it("drops a slow reply that lands after a newer, quicker one", async () => {
    const ticket = latestOnly();
    const applied: string[] = [];
    const request = async (answer: string, delayMs: number) => {
      const isCurrent = ticket();
      await new Promise((resolve) => setTimeout(resolve, delayMs));
      if (isCurrent()) applied.push(answer);
    };
    await Promise.all([request("old keystroke", 20), request("new keystroke", 0)]);
    expect(applied).toEqual(["new keystroke"]);
  });

  it("keeps separate guards independent", () => {
    const search = latestOnly();
    const counts = latestOnly();
    const s = search();
    counts();
    expect(s()).toBe(true);
  });
});
