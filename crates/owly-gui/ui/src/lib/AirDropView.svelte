<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount } from "svelte";
  import type { Peer } from "./api";
  import Avatar from "./Avatar.svelte";
  import type { Model } from "./model.svelte";
  import PersonCard from "./PersonCard.svelte";
  import VisibilityFooter from "./VisibilityFooter.svelte";

  interface Props {
    model: Model;
    onchoose: (peer: Peer) => void;
  }

  let { model, onchoose }: Props = $props();
  let hovered = $state<string | null>(null);
  let hint = $state<string | null>(null);

  let status = $derived(model.status);
  let on = $derived(status?.browsing ?? false);

  /** The person under a point of the window, if any. Positions arrive in
   * physical pixels. */
  function personAt(x: number, y: number): string | null {
    const ratio = window.devicePixelRatio || 1;
    const el = document.elementFromPoint(x / ratio, y / ratio);
    return (
      el?.closest<HTMLElement>("[data-peer-id]")?.dataset["peerId"] ?? null
    );
  }

  onMount(() => {
    const stop = getCurrentWebview().onDragDropEvent((event) => {
      const drag = event.payload;
      if (drag.type === "leave") {
        hovered = null;
      } else if (drag.type === "over" || drag.type === "enter") {
        hovered = personAt(drag.position.x, drag.position.y);
      } else {
        const target = personAt(drag.position.x, drag.position.y);
        hovered = null;
        if (target) {
          hint = null;
          void model.send(target, drag.paths, []);
        } else {
          hint = "Drop the files onto a person to send them.";
        }
      }
    });
    return () => void stop.then((off) => off());
  });
</script>

<div class="view">
  <header>
    <h1 class="visually-hidden">AirDrop</h1>
    {#if status}
      <div class="me">
        <Avatar name={status.name} size={64} badge />
        <div>
          <p class="as">Use AirDrop as</p>
          <p class="who">{status.name}</p>
        </div>
        <div class="power">
          {#if on}
            <button
              type="button"
              class="plain"
              disabled={model.busy}
              onclick={() => model.turnOff()}
            >
              Turn off
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </header>

  <section class="people" aria-labelledby="people-heading">
    {#if on && model.peers.length > 0}
      <h2 id="people-heading">People</h2>
      <ul>
        {#each model.peers as peer (peer.id)}
          <li>
            <PersonCard
              {peer}
              transfer={model.transfers[peer.id]}
              dropping={hovered === peer.id}
              {onchoose}
            />
          </li>
        {/each}
      </ul>
    {:else}
      <h2 id="people-heading" class="visually-hidden">People</h2>
      <div class="empty">
        {#if on}
          <p class="title">No People Found</p>
          <p class="sub">There is no one nearby to share with.</p>
        {:else}
          <p class="title">AirDrop is Off</p>
          <p class="sub">Turn it on to find people nearby and to be found.</p>
          <button
            type="button"
            class="plain primary"
            disabled={model.busy}
            onclick={() => model.turnOn()}
          >
            {model.busy ? "Turning on…" : "Turn On AirDrop"}
          </button>
        {/if}
      </div>
    {/if}
    {#if hint}
      <p class="drop-hint">{hint}</p>
    {/if}
  </section>

  {#if status}
    <VisibilityFooter
      {status}
      busy={model.busy}
      onchange={(m) => model.setVisibility(m)}
    />
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    padding: 18px 24px 14px;
    border-bottom: 1px solid var(--line);
  }

  .me {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .as {
    margin: 0;
    color: var(--text-quiet);
    font-size: 1.05em;
  }

  .who {
    margin: 0;
    font-size: 1.35em;
    font-weight: 600;
  }

  .power {
    margin-left: auto;
  }

  .people {
    flex: 1;
    overflow: auto;
    padding: 16px 24px;
  }

  h2 {
    margin-bottom: 10px;
    color: var(--text-quiet);
    font-size: 1.2em;
  }

  ul {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .empty {
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 6px;
    height: 100%;
    min-height: 200px;
    text-align: center;
  }

  .title {
    margin: 0;
    font-size: 1.6em;
    font-weight: 700;
  }

  .sub {
    margin: 0 0 8px;
    color: var(--text-quiet);
    font-size: 1.05em;
  }

  .drop-hint {
    color: var(--text-quiet);
  }
</style>
