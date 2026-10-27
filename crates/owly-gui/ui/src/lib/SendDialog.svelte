<script lang="ts">
  import type { Peer, SelectionInfo } from "./api";
  import type { Model } from "./model.svelte";

  interface Props {
    model: Model;
    peer: Peer | null;
    onclose: () => void;
  }

  let { model, peer, onclose }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let kind = $state<"files" | "link">("files");
  let selection = $state<SelectionInfo | null>(null);
  let link = $state("");
  let problem = $state<string | null>(null);
  let sent = false;

  let ready = $derived(
    kind === "files" ? selection !== null : link.trim().length > 0,
  );

  $effect(() => {
    if (peer && dialog && !dialog.open) {
      kind = "files";
      selection = null;
      link = "";
      problem = null;
      sent = false;
      dialog.showModal();
    }
  });

  // The backend keeps what was picked; give it back if nothing was sent.
  function closed(): void {
    if (selection && !sent) void model.discard(selection.id);
    selection = null;
    onclose();
  }

  async function choose(directory: boolean): Promise<void> {
    problem = null;
    const picked = await model.choose(selection?.id ?? null, directory);
    if (picked) selection = picked;
    problem = model.problem;
  }

  async function send(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!peer || !ready) return;
    const ok =
      kind === "files" && selection
        ? await model.sendSelection(peer.id, selection.id)
        : await model.sendLink(peer.id, link.trim());
    if (ok) {
      sent = true;
      dialog?.close();
    } else {
      problem = model.problem;
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="send-title" onclose={closed}>
  {#if peer}
    <form onsubmit={send}>
      <h2 id="send-title">Send to {peer.name}</h2>

      <fieldset>
        <legend class="visually-hidden">What to send</legend>
        <label
          ><input type="radio" bind:group={kind} value="files" /> Files</label
        >
        <label
          ><input type="radio" bind:group={kind} value="link" /> A link</label
        >
      </fieldset>

      {#if kind === "files"}
        <div class="row">
          <button type="button" class="plain" onclick={() => choose(false)}
            >Add Files…</button
          >
          <button type="button" class="plain" onclick={() => choose(true)}
            >Add Folder…</button
          >
        </div>
        {#if selection}
          <ul aria-label="Chosen items">
            {#each selection.names as name, i (i)}
              <li>{name}</li>
            {/each}
            {#if selection.more > 0}
              <li class="quiet">and {selection.more} more</li>
            {/if}
          </ul>
        {:else}
          <p class="quiet">
            Choose what to send, or close this and drop files onto the person.
          </p>
        {/if}
      {:else}
        <label class="field" for="link">Link</label>
        <input
          id="link"
          type="url"
          inputmode="url"
          placeholder="https://"
          bind:value={link}
          aria-describedby={problem ? "send-problem" : undefined}
        />
      {/if}

      {#if problem}
        <p id="send-problem" class="problem" role="alert">{problem}</p>
      {/if}

      <div class="actions">
        <button type="button" class="plain" onclick={() => dialog?.close()}
          >Cancel</button
        >
        <button type="submit" class="plain primary" disabled={!ready}
          >Send</button
        >
      </div>
    </form>
  {/if}
</dialog>

<style>
  dialog {
    width: min(460px, calc(100vw - 32px));
    padding: 20px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--bg);
    color: var(--text);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }

  form {
    display: grid;
    gap: 12px;
  }

  fieldset {
    display: flex;
    gap: 18px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  .row,
  .actions {
    display: flex;
    gap: 8px;
  }

  .actions {
    justify-content: flex-end;
  }

  ul {
    margin: 0;
    padding-left: 18px;
    max-height: 160px;
    overflow: auto;
    overflow-wrap: anywhere;
  }

  input[type="url"] {
    padding: 7px 9px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg-raised);
  }

  .quiet {
    margin: 0;
    color: var(--text-quiet);
  }

  .problem {
    margin: 0;
    color: var(--danger);
  }
</style>
