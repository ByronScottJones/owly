<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Peer } from "./api";
  import { errorText } from "./api";
  import { baseName } from "./format";
  import type { Model } from "./model.svelte";

  interface Props {
    model: Model;
    peer: Peer | null;
    onclose: () => void;
  }

  let { model, peer, onclose }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let kind = $state<"files" | "link">("files");
  let paths = $state<string[]>([]);
  let link = $state("");
  let problem = $state<string | null>(null);

  let ready = $derived(
    kind === "files" ? paths.length > 0 : link.trim().length > 0,
  );

  $effect(() => {
    if (peer && dialog && !dialog.open) {
      kind = "files";
      paths = [];
      link = "";
      problem = null;
      dialog.showModal();
    }
  });

  async function choose(directory: boolean): Promise<void> {
    problem = null;
    try {
      const picked = await open({ multiple: !directory, directory });
      const chosen =
        picked === null ? [] : Array.isArray(picked) ? picked : [picked];
      paths = [...new Set([...paths, ...chosen])];
    } catch (e: unknown) {
      problem = errorText(e);
    }
  }

  async function send(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!peer || !ready) return;
    const sent =
      kind === "files"
        ? await model.send(peer.id, paths, [])
        : await model.send(peer.id, [], [link.trim()]);
    if (sent) dialog?.close();
    else problem = model.problem;
  }
</script>

<dialog bind:this={dialog} aria-labelledby="send-title" {onclose}>
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
        {#if paths.length > 0}
          <ul aria-label="Chosen items">
            {#each paths as path (path)}
              <li title={path}>{baseName(path)}</li>
            {/each}
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
