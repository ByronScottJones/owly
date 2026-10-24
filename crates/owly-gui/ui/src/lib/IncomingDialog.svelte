<script lang="ts">
  import type { IncomingRequest } from "./api";
  import { formatSize } from "./format";

  interface Props {
    request: IncomingRequest | undefined;
    queued: number;
    onanswer: (id: number, accept: boolean) => void;
  }

  let { request, queued, onanswer }: Props = $props();
  let dialog = $state<HTMLDialogElement>();

  // The sender's words are shown as text and never opened (SEC-4).
  let count = $derived(
    request
      ? request.offer.type === "files"
        ? request.offer.files.length
        : request.offer.urls.length
      : 0,
  );
  let noun = $derived(request?.offer.type === "urls" ? "link" : "item");

  $effect(() => {
    if (!dialog) return;
    if (request && !dialog.open) dialog.showModal();
    if (!request && dialog.open) dialog.close();
  });

  function decline(event: Event): void {
    // Escape declines: it must not leave the request hanging.
    event.preventDefault();
    if (request) onanswer(request.id, false);
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="incoming-title"
  aria-describedby="incoming-list"
  oncancel={decline}
>
  {#if request}
    <h2 id="incoming-title">
      “{request.senderName}” would like to share {count}
      {noun}{count === 1 ? "" : "s"}
    </h2>
    <p class="quiet">{request.senderModel}</p>
    <ul id="incoming-list">
      {#if request.offer.type === "files"}
        {#each request.offer.files as file, i (i)}
          <li>
            <span class="file">{file.name}</span>
            <span class="quiet"
              >{[file.kind, formatSize(file.size)]
                .filter(Boolean)
                .join(", ")}</span
            >
          </li>
        {/each}
      {:else}
        {#each request.offer.urls as url, i (i)}
          <li><span class="file">{url}</span></li>
        {/each}
        <li class="quiet">Links are shown here, and never opened for you.</li>
      {/if}
    </ul>
    {#if queued > 0}
      <p class="quiet">{queued} more waiting.</p>
    {/if}
    <p class="quiet">This is declined by itself after 15 seconds.</p>
    <div class="actions">
      <button
        type="button"
        class="plain"
        onclick={() => onanswer(request.id, false)}>Decline</button
      >
      <button
        type="button"
        class="plain primary"
        onclick={() => onanswer(request.id, true)}>Accept</button
      >
    </div>
  {/if}
</dialog>

<style>
  dialog {
    width: min(480px, calc(100vw - 32px));
    padding: 20px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--bg);
    color: var(--text);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }

  h2 {
    font-size: 1.15em;
  }

  ul {
    max-height: 200px;
    margin: 10px 0;
    padding-left: 18px;
    overflow: auto;
  }

  .file {
    overflow-wrap: anywhere;
  }

  .quiet {
    margin: 4px 0;
    color: var(--text-quiet);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
</style>
