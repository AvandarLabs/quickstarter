# SQL rules

- If a `supabase-postgres-best-practices` skill is available, use it for query,
  schema, and performance guidance.

## Naming

- `snake_case` everywhere.
- Tables: plural (`profiles`, not `profile`).
- Functions: namespace-prefixed.
  - `util__*` for utilities.
  - `table_name__*` for table-specific (e.g. `profiles__get_active`).
- Triggers: `tr__table_name__*`.

## Row level security (RLS)

- Threat model: the browser holds the Supabase **publishable (anon) key**. Any
  authenticated user can hand-craft PostgREST calls (JSON, filters, verbs).
  RLS is the only authorization layer, not client code.
- Tie every sensitive column to the id that scopes ownership **on the row**
  (or a join resolving to it). Never trust a client-supplied scoping id unless
  bound to the tuple PostgREST is mutating.
- Helper functions take the scoping id **from the row expression**, not
  from the JWT or a client value.
- `UPDATE` policies must include `WITH CHECK` constraining the **new** row
  the same way as `USING` constrains the old row: otherwise users can
  rewrite a row into a scope they don't own.

## Tests

- **RLS policies must always be tested.** Asserting the happy path
  (allowed user can read their row) is not enough. For every policy, also
  assert the **negative** cases:
  - A user outside the scope **cannot** `SELECT` the row.
  - A user outside the scope **cannot** `INSERT` a row claiming that
    scoping id (covers `WITH CHECK` on inserts).
  - A user outside the scope **cannot** `UPDATE` the row, **and** an
    in-scope user cannot `UPDATE` the row to move it into another
    scope (covers `WITH CHECK` on updates).
  - A user outside the scope **cannot** `DELETE` the row.
  - Lower-privileged in-scope roles cannot perform actions reserved for
    higher roles (e.g. viewer cannot write).
- Test against rows the current user should **not** see, not just rows they
  should. A policy that silently returns zero rows for unauthorized reads
  is correct; a policy that lets an unauthorized write succeed is a breach.
