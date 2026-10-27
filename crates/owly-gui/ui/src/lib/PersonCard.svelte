<script lang="ts">
  import type { Peer, TransferUpdate } from "./api";
  import Avatar from "./Avatar.svelte";
  import { percent, transferLabel } from "./format";

  interface Props {
    peer: Peer;
    transfer?: TransferUpdate;
    dropping?: boolean;
    onchoose: (peer: Peer) => void;
  }

  let { peer, transfer, dropping = false, onchoose }: Props = $props();
  let label = $derived(transfer ? transferLabel(transfer) : "");
  let done = $derived(transfer ? percent(transfer) : null);
</script>

<button
  type="button"
  class="person"
  class:dropping
  data-peer-id={peer.id}
  onclick={() => onchoose(peer)}
>
  <Avatar name={peer.name} size={72} />
  <span class="name"
    ><span class="visually-hidden">Send to </span>{peer.name}</span
  >
  <span class="state">{label}</span>
  {#if transfer?.state === "progress" && done !== null}
    <progress value={done} max="100" aria-label={`Sending to ${peer.name}`}
    ></progress>
  {/if}
</button>

<style>
  .person {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 148px;
    min-height: 140px;
    padding: 12px 8px;
    border: 2px solid transparent;
    border-radius: var(--radius);
    background: none;
    cursor: pointer;
    text-align: center;
  }

  .person:hover {
    background: var(--bg-raised);
  }

  .person.dropping {
    background: var(--drop);
    border-color: var(--accent);
  }

  .name {
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .state {
    color: var(--text-quiet);
    font-size: 0.9em;
    min-height: 1.3em;
    overflow-wrap: anywhere;
  }

  progress {
    width: 100%;
    accent-color: var(--accent);
  }
</style>
