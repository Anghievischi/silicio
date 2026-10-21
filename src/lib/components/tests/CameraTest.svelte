<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  let video = $state<HTMLVideoElement>();
  let stream: MediaStream | null = null;
  let error = $state<string | null>(null);
  let width = $state<number | null>(null);
  let height = $state<number | null>(null);
  let fps = $state<number | null>(null);
  let measured = $state<number | null>(null);

  onMount(async () => {
    if (!navigator.mediaDevices?.getUserMedia) {
      error = t().tests.cameraError("getUserMedia");
      return;
    }
    try {
      // Pede a maior resolução; a câmera devolve a mais próxima que suporta.
      stream = await navigator.mediaDevices
        .getUserMedia({ video: { width: { ideal: 3840 }, height: { ideal: 2160 } }, audio: false })
        .catch(() => navigator.mediaDevices.getUserMedia({ video: true, audio: false }));
      if (video) video.srcObject = stream;
      const settings = stream.getVideoTracks()[0]?.getSettings();
      width = settings?.width ?? null;
      height = settings?.height ?? null;
      fps = settings?.frameRate ? Math.round(settings.frameRate) : null;
      measure();
    } catch (e) {
      error = t().tests.cameraError(e instanceof Error ? e.message : String(e));
    }
  });

  // Conta os quadros que chegam de fato, quando o navegador permite.
  function measure() {
    const v = video as (HTMLVideoElement & { requestVideoFrameCallback?: (cb: () => void) => number }) | undefined;
    if (!v?.requestVideoFrameCallback) return;
    let frames = 0;
    let since = performance.now();
    const tick = () => {
      if (!stream) return;
      frames++;
      const now = performance.now();
      if (now - since >= 1000) {
        measured = Math.round((frames * 1000) / (now - since));
        frames = 0;
        since = now;
      }
      v.requestVideoFrameCallback!(tick);
    };
    v.requestVideoFrameCallback(tick);
  }

  onDestroy(() => {
    stream?.getTracks().forEach((track) => track.stop());
    stream = null;
  });
</script>

<p class="muted">{t().tests.cameraHint}</p>
{#if error}
  <p class="error">{error}</p>
{:else}
  <!-- svelte-ignore a11y_media_has_caption -->
  <video bind:this={video} autoplay playsinline muted></video>
  <div class="stats">
    <div><span class="muted">{t().tests.resolution}</span><strong class="num">{width && height ? `${width} × ${height}` : "—"}</strong></div>
    <div><span class="muted">{t().tests.fps}</span><strong class="num">{measured ?? fps ?? "—"}</strong></div>
  </div>
{/if}

<style>
  video {
    width: 100%;
    max-height: 55vh;
    border-radius: var(--radius-sm);
    background: #000;
    transform: scaleX(-1);
  }

  .stats {
    display: flex;
    gap: 32px;
    margin-top: 12px;
  }

  .stats div {
    display: flex;
    flex-direction: column;
  }

  .stats strong {
    font-size: 22px;
  }
</style>
