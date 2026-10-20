<script lang="ts">
  let { kind }: { kind: string } = $props();

  // Ícones de traço simples (24×24) para cada tipo de periférico.
  const PATHS: Record<string, string> = {
    keyboard: "M4 6h16a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z M6 10h.01 M10 10h.01 M14 10h.01 M18 10h.01 M7 14h10",
    mouse: "M12 3a5 5 0 0 1 5 5v8a5 5 0 0 1-10 0V8a5 5 0 0 1 5-5Z M12 7v4",
    camera: "M12 5a6 6 0 1 1 0 12a6 6 0 0 1 0-12Z M12 9a2 2 0 1 1 0 4a2 2 0 0 1 0-4Z M8 21h8 M12 17v4",
    audio: "M4 15v-3a8 8 0 0 1 16 0v3 M4 14h3v6H5a1 1 0 0 1-1-1Z M20 14h-3v6h2a1 1 0 0 0 1-1Z",
    bluetooth: "M7 7l10 10-5 4V3l5 4L7 17",
    gamepad: "M7 8h10a5 5 0 0 1 0 10c-1.5 0-2.5-1-3-2h-4c-.5 1-1.5 2-3 2A5 5 0 0 1 7 8Z M7 11v4 M5 13h4 M15.5 12h.01 M17.5 14h.01",
    storage: "M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z M3 14h18 M17 17h.01",
    receiver: "M9 3h6v10H9Z M7 13h10v6a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2Z M11 6h.01 M13 6h.01",
    network: "M2 9a15 15 0 0 1 20 0 M5 12.5a10 10 0 0 1 14 0 M8.5 16a5 5 0 0 1 7 0 M12 19.5h.01",
    smartcard: "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Z M7 9h4v4H7Z",
    display: "M4 4h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Z M8 20h8 M12 16v4",
    hid: "M6 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z M12 9a3 3 0 1 1 0 6a3 3 0 0 1 0-6Z",
  };
</script>

<svg viewBox="0 0 24 24" aria-hidden="true">
  <path d={PATHS[kind] ?? PATHS.hid} />
</svg>

<style>
  svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
