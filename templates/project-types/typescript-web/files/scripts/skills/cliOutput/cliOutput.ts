import { Acclimate } from "@avandar/acclimate";

/**
 * Semantic output helpers.
 *
 * Acclimate has no theme of its own yet, so the colors live here and every
 * message in the skills tooling goes through one of these functions. That
 * keeps the CLI's voice consistent and makes tests able to spy on a single
 * logger.
 */

/** Prints a blank line followed by a bright section heading. */
export function printHeading(message: string): void {
  Acclimate.log(`\n|bright_white|${message}|reset|`);
}

/** Prints a message verbatim, colors and all. Used for pre-built tables. */
export function printBlock(message: string): void {
  Acclimate.log(message);
}

/** Prints a confirmation in green. */
export function printSuccess(message: string): void {
  Acclimate.log(`|green|${message}|reset|`);
}

/** Prints a caution in yellow: something needs attention but nothing broke. */
export function printWarning(message: string): void {
  Acclimate.log(`|yellow|${message}|reset|`);
}

/** Prints a failure in red. */
export function printError(message: string): void {
  Acclimate.log(`|red|${message}|reset|`);
}

/** Prints supporting detail in gray. */
export function printInfo(message: string): void {
  Acclimate.log(`|gray|${message}|reset|`);
}
