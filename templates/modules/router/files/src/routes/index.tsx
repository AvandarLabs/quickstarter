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
        This template is up and running. Edit{" "}
        <Text span ff="monospace">
          src/routes/index.tsx
        </Text>{" "}
        to get started.
      </Text>
    </Stack>
  );
}
