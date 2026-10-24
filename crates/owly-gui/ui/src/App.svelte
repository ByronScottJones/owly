<script lang="ts">
  import { onMount } from "svelte";
  import {
    acquireRadio,
    errorText,
    getStatus,
    on,
    startBrowse,
    type Peer,
    type Status,
  } from "./lib/api";

  let status = $state<Status | null>(null);
  let peers = $state<Peer[]>([]);
  let problem = $state<string | null>(null);

  async function refresh(): Promise<void> {
    try {
      status = await getStatus();
    } catch (e: unknown) {
      problem = errorText(e);
    }
  }

  async function look(): Promise<void> {
    problem = null;
    try {
      await acquireRadio();
      await startBrowse();
    } catch (e: unknown) {
      problem = errorText(e);
    }
  }

  onMount(() => {
    void refresh();
    const stops = [
      on<Peer[]>("peers", (found) => (peers = found)),
      on<null>("status-changed", () => void refresh()),
    ];
    return () => {
      for (const stop of stops) void stop.then((off) => off());
    };
  });
</script>

<main>
  <h1>Owly</h1>
  {#if status}
    <p>
      Sharing as {status.name} on {status.interface}. Radio: {status.radio
        .state}.
    </p>
  {/if}
  <button type="button" onclick={look}>Look for people</button>
  {#if problem}
    <p role="alert">{problem}</p>
  {/if}
  <ul aria-label="People nearby">
    {#each peers as peer (peer.id)}
      <li>{peer.name}</li>
    {:else}
      <li>No people found</li>
    {/each}
  </ul>
</main>
