import { Stack, Text, Title } from "@mantine/core";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <Stack align="center" justify="center" mih="100vh" gap="xs">
      <Title order={1}>Hello, world</Title>
      <Text c="dimmed">
        This TanStack Start template is up and running. Edit{" "}
        <Text span ff="monospace">
          src/routes/index.tsx
        </Text>{" "}
        to get started. Server functions are available via{" "}
        <Text span ff="monospace">
          createServerFn
        </Text>{" "}
        from{" "}
        <Text span ff="monospace">
          @tanstack/react-start
        </Text>
        .
      </Text>
    </Stack>
  );
}
