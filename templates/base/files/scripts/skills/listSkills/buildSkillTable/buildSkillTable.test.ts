import { describe, expect, it } from "vitest";
import { buildSkillTable } from "./buildSkillTable";
import type { SkillListingRow } from "../../skills.types";

const ROWS: SkillListingRow[] = [
  {
    name: "brainstorming",
    manager: "skills",
    source: "obra/superpowers",
    agents: ["Claude Code"],
    isInstalled: true,
  },
  {
    name: "impeccable",
    manager: "impeccable",
    source: "pbakaus/impeccable",
    agents: ["Claude Code"],
    isInstalled: true,
  },
  {
    name: "writing-plans",
    manager: "skills",
    source: "obra/superpowers",
    agents: [],
    isInstalled: false,
  },
];

/** Strips Acclimate's `|color|` tokens so widths can be measured. */
function withoutColorTokens(text: string): string {
  return text.replaceAll(/\|[a-z_]+\|/g, "");
}

describe("buildSkillTable", () => {
  it("prints a header and one line per skill", () => {
    const lines = buildSkillTable(ROWS).split("\n");

    expect(withoutColorTokens(lines[0] ?? "")).toContain("SKILL");
    expect(withoutColorTokens(lines[0] ?? "")).toContain("MANAGER");
    expect(withoutColorTokens(lines[0] ?? "")).toContain("SOURCE");
    expect(lines).toHaveLength(ROWS.length + 1);
  });

  it("names every skill and its manager and source", () => {
    const table = withoutColorTokens(buildSkillTable(ROWS));

    expect(table).toContain("brainstorming");
    expect(table).toContain("impeccable");
    expect(table).toContain("pbakaus/impeccable");
    expect(table).toContain("obra/superpowers");
  });

  it("aligns the columns across rows", () => {
    const lines = buildSkillTable(ROWS).split("\n").map(withoutColorTokens);
    const managerColumnStarts = lines.map((line) => {
      return line.toLowerCase().indexOf("skills");
    });

    // Row 0 is the header ("SKILL"), so compare the two `skills`-managed rows.
    expect(managerColumnStarts[1]).toBe(managerColumnStarts[3]);
  });

  it("marks a skill that is not installed", () => {
    const table = withoutColorTokens(buildSkillTable(ROWS));

    expect(table).toMatch(/writing-plans.*not installed/);
    expect(table).not.toMatch(/brainstorming.*not installed/);
  });

  it("returns an empty string when there is nothing to show", () => {
    expect(buildSkillTable([])).toBe("");
  });
});
