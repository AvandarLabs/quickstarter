import "@mantine/core/styles.css";
import "@/index.css";
import {
  ColorSchemeScript,
  MantineProvider,
  mantineHtmlProps,
} from "@mantine/core";
import {
  createRootRoute,
  HeadContent,
  Outlet,
  Scripts,
} from "@tanstack/react-router";
import type { ReactNode } from "react";
import { theme } from "@/theme";

/**
 * The root route. It defines the full HTML document (there is no `index.html`
 * in a TanStack Start app) and wraps every page in the Mantine provider so
 * server-side rendering and client hydration share the same theme. Add
 * app-wide chrome (headers, navigation, providers) inside `<RootDocument>`.
 */
export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "{{PROJECT_NAME}}" },
    ],
  }),
  component: RootComponent,
});

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  );
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html {...mantineHtmlProps}>
      <head>
        <ColorSchemeScript />
        <HeadContent />
      </head>
      <body>
        <MantineProvider theme={theme}>{children}</MantineProvider>
        <Scripts />
      </body>
    </html>
  );
}
