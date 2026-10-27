<script lang="ts">
  import type { Arrival } from "./model.svelte";

  interface Props {
    arrivals: Arrival[];
    receiveDir: string;
    ondismiss: (key: string) => void;
  }

  let { arrivals, receiveDir, ondismiss }: Props = $props();

  const headings = {
    files: "Received",
    urls: "Received a link",
    failed: "A transfer failed",
  } as const;

  async function copy(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Selecting the text by hand still works.
    }
  }
</script>

<div class="arrivals" role="status" aria-live="polite">
  {#each arrivals as arrival (arrival.key)}
    <section class:failed={arrival.kind === "failed"}>
      <h2>{headings[arrival.kind]}</h2>
      <ul>
        {#each arrival.lines as line, i (i)}
          <li>
            <span class="text">{line}</span>
            {#if arrival.kind === "urls"}
              <button type="button" class="plain" onclick={() => copy(line)}
                >Copy</button
              >
            {/if}
          </li>
        {/each}
      </ul>
      {#if arrival.kind === "files"}
        <p class="where">In {receiveDir}</p>
      {/if}
      <button type="button" class="plain" onclick={() => ondismiss(arrival.key)}
        >Dismiss</button
      >
    </section>
  {/each}
</div>

<style>
  .arrivals {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: grid;
    gap: 10px;
    width: min(360px, calc(100vw - 32px));
    z-index: 5;
  }

  section {
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg-raised);
    box-shadow: 0 6px 20px rgb(0 0 0 / 0.2);
  }

  section.failed {
    border-color: var(--danger);
  }

  h2 {
    margin-bottom: 4px;
    font-size: 1em;
  }

  ul {
    margin: 0 0 6px;
    padding-left: 18px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .text {
    flex: 1;
    overflow-wrap: anywhere;
  }

  .where {
    margin: 0 0 8px;
    color: var(--text-quiet);
    font-size: 0.9em;
    overflow-wrap: anywhere;
  }
</style>
