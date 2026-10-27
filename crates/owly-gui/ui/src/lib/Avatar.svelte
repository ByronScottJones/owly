<script lang="ts">
  import { hueOf, initials } from "./format";

  interface Props {
    name: string;
    size?: number;
    /** The small radio badge Finder puts on your own picture. */
    badge?: boolean;
  }

  let { name, size = 64, badge = false }: Props = $props();
</script>

<span
  class="avatar"
  style:--size="{size}px"
  style:--hue={hueOf(name)}
  aria-hidden="true"
>
  <span class="letters" data-initials={initials(name)}></span>
  {#if badge}
    <svg class="badge" viewBox="0 0 24 24" focusable="false">
      <rect width="24" height="24" rx="6" fill="#0b5fd3" />
      <circle cx="12" cy="12" r="2.2" fill="#fff" />
      <path
        d="M7.8 7.8a6 6 0 0 0 0 8.4M16.2 7.8a6 6 0 0 1 0 8.4"
        fill="none"
        stroke="#fff"
        stroke-width="1.6"
        stroke-linecap="round"
      />
    </svg>
  {/if}
</span>

<style>
  .avatar {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background: hsl(var(--hue) 42% var(--avatar-l));
    color: #fff;
    font-weight: 600;
    font-size: calc(var(--size) * 0.36);
    user-select: none;
  }

  /* Generated content: the initials are decoration, not text to read. */
  .letters::before {
    content: attr(data-initials);
  }

  .badge {
    position: absolute;
    right: -4px;
    bottom: -4px;
    width: calc(var(--size) * 0.36);
    height: calc(var(--size) * 0.36);
  }
</style>
