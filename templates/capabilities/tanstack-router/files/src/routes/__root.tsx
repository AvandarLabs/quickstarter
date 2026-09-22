import { createRootRoute, Outlet } from "@tanstack/react-router";

/**
 * The root route. Every page renders inside this layout via `<Outlet />`.
 * Add app-wide chrome (headers, navigation, providers) here as needed.
 */
export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return <Outlet />;
}
