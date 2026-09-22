import type { SkillListingRow } from "../../skills.types";

/**
 * Column headers. Padding is computed on the visible text and the Acclimate
 * color tokens are wrapped around the padded cell, so tokens never shift a
 * column: they collapse to zero width when the message is printed.
 */
const HEADERS = ["SKILL", "MANAGER", "SOURCE"] as const;

const MISSING_NOTE = "not installed";
const COLUMN_GAP = "  ";

function _padCell(text: string, width: number): string {
  return text.padEnd(width, " ");
}

function _createCells(row: Readonly<SkillListingRow>): string[] {
  return [row.name, row.manager, row.source ?? "-"];
}

function _colorizeRow(cells: readonly string[], isInstalled: boolean): string {
  const [name, manager, source] = cells;
  const nameColor = isInstalled ? "|bright_white|" : "|yellow|";
  const note = isInstalled
    ? ""
    : `${COLUMN_GAP}|yellow|(${MISSING_NOTE})|reset|`;
  return (
    `${nameColor}${name}|reset|${COLUMN_GAP}` +
    `|cyan|${manager}|reset|${COLUMN_GAP}` +
    `|gray|${source}|reset|${note}`
  );
}

/**
 * Renders the skill listing as an aligned, colored table.
 *
 * @param rows The skills to show, already ordered.
 * @returns The table as a string, or an empty string when there are no rows.
 */
export function buildSkillTable(rows: readonly SkillListingRow[]): string {
  if (rows.length === 0) {
    return "";
  }

  const allCells = [[...HEADERS], ...rows.map(_createCells)];
  const columnWidths = HEADERS.map((_header, columnIndex) => {
    return Math.max(
      ...allCells.map((cells) => {
        return (cells[columnIndex] ?? "").length;
      }),
    );
  });

  const paddedRows = allCells.map((cells) => {
    return cells.map((cell, columnIndex) => {
      return _padCell(cell, columnWidths[columnIndex] ?? 0);
    });
  });

  const [headerCells, ...bodyCells] = paddedRows;
  const headerLine = `|gray|${(headerCells ?? []).join(COLUMN_GAP)}|reset|`;
  const bodyLines = bodyCells.map((cells, rowIndex) => {
    return _colorizeRow(cells, rows[rowIndex]?.isInstalled ?? false);
  });

  return [headerLine, ...bodyLines].join("\n");
}
