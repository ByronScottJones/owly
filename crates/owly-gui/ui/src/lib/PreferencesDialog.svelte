<script lang="ts">
  import type { Status } from "./api";

  interface Props {
    status: Status | null;
    open: boolean;
    onclose: () => void;
    onopenlinks: (on: boolean) => Promise<void>;
  }

  let { status, open, onclose, onopenlinks }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let busy = $state(false);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });

  // Turning links on asks in a dialog of the backend's own, and "Cancel"
  // there leaves things as they were, so the box follows what is in force.
  async function change(box: HTMLInputElement): Promise<void> {
    busy = true;
    try {
      await onopenlinks(box.checked);
    } finally {
      busy = false;
      box.checked = status?.settings.openLinks ?? false;
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="preferences-title" {onclose}>
  <h2 id="preferences-title">Preferences</h2>
  {#if status}
    <fieldset>
      <legend>Links from other devices</legend>
      <label>
        <input
          type="checkbox"
          checked={status.settings.openLinks}
          disabled={busy || status.settings.openLinksFixed}
          aria-describedby="open-links-hint"
          onchange={(e) => void change(e.currentTarget)}
        />
        Show an Open button on links I receive
      </label>
      <p id="open-links-hint" class="quiet">
        A link is never opened by itself, only when you click Open. Check each
        address first: the window warns about addresses with characters that are
        not plain ASCII, but it cannot tell you whether a site is safe.
        {#if status.settings.openLinksFixed}
          This is set by the OWLY_OPEN_LINKS environment variable, so it can't
          be changed here.
        {/if}
      </p>
    </fieldset>
    <p class="quiet">Settings are kept in {status.settings.file}.</p>
  {/if}
  <div class="actions">
    <button type="button" class="plain primary" onclick={onclose}>Close</button>
  </div>
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

  fieldset {
    margin: 10px 0;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }

  legend {
    padding: 0 6px;
    color: var(--accent);
  }

  .quiet {
    margin: 6px 0 0;
    color: var(--text-quiet);
    font-size: 0.9em;
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 14px;
  }
</style>
