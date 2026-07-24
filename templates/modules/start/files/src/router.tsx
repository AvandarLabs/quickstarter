import { createRouter } from "@tanstack/react-router";
import { routeTree } from "@/routeTree.gen";

/**
 * Creates the application router. TanStack Start calls `getRouter` to build a
 * fresh router instance for every request (server) and once on the client. The
 * route tree is generated automatically by Start from the files in
 * `src/routes`. Do not edit `routeTree.gen.ts` by hand.
 */
export function getRouter() {
  const router = createRouter({
    routeTree,
    scrollRestoration: true,
  });

  return router;
}

// Register the router instance for full type safety across the app.
declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
