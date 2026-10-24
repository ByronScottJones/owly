<script lang="ts">
  import type { Status, Visibility } from "./api";

  interface Props {
    status: Status;
    busy: boolean;
    onchange: (mode: Visibility) => void;
  }

  let { status, busy, onchange }: Props = $props();

  const hints: Record<Visibility, string> = {
    everyone: "Anyone nearby can see this computer and ask to send you files.",
    contacts_only: "Only people in your contacts can see this computer.",
    off: "This computer is hidden, and can't receive.",
  };
</script>

<footer>
  <p class="about">AirDrop lets you share instantly with people nearby.</p>
  <div class="visibility">
    <label for="visibility">Allow me to be discovered by:</label>
    <select
      id="visibility"
      aria-describedby="visibility-hint"
      value={status.visibility}
      disabled={busy}
      onchange={(e) => onchange(e.currentTarget.value as Visibility)}
    >
      <option value="everyone">Everyone</option>
      <option value="contacts_only" disabled={!status.contactsOnlyAvailable}>
        Contacts Only{status.contactsOnlyAvailable ? "" : " (unavailable)"}
      </option>
      <option value="off">Receiving Off</option>
    </select>
  </div>
  <p id="visibility-hint" class="hint">
    {hints[status.visibility]}
    {#if !status.contactsOnlyAvailable}
      Contacts Only needs an Apple identity, which this computer doesn't have.
    {/if}
  </p>
</footer>

<style>
  footer {
    padding: 14px 24px 16px;
    border-top: 1px solid var(--line);
  }

  .about {
    margin: 0 0 6px;
  }

  .visibility {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  label {
    color: var(--accent);
  }

  select {
    padding: 5px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg-raised);
  }

  .hint {
    margin: 6px 0 0;
    color: var(--text-quiet);
    font-size: 0.9em;
  }
</style>
