import "@mantine/core/styles.css";
import "@/index.css";
import { MantineProvider } from "@mantine/core";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { router } from "@/router";
import { theme } from "@/theme";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element with id 'root' was not found in the DOM.");
}

createRoot(rootElement).render(
  <StrictMode>
    <MantineProvider theme={theme}>
      <RouterProvider router={router} />
    </MantineProvider>
  </StrictMode>,
);
