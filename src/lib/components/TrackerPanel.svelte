<script lang="ts">
  import { openExternalUrl, revealPath } from "$lib/api/opener";
  import type { Tracker } from "$lib/api/types";
  import { trackerFields } from "$lib/trackers";

  let { tracker, onerror }: { tracker: Tracker; onerror?: (m: string) => void } = $props();

  let fields = $derived(trackerFields(tracker));

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.warn("clipboard write failed", err);
    }
  }

  async function open(url: string) {
    try {
      await openExternalUrl(url);
    } catch (err) {
      console.warn("open external url failed", err);
      onerror?.((err as Error).message);
    }
  }

  async function reveal(path: string) {
    try {
      await revealPath(path);
    } catch (err) {
      console.warn("reveal path failed", err);
      onerror?.((err as Error).message);
    }
  }

  const iconBtn =
    "text-phos-faint hover:text-phos text-[13px] uppercase tracking-wide font-display";
</script>

{#if fields.length === 0}
  <p class="text-sm text-phos-dim">No details available.</p>
{:else}
  <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-2 text-sm">
    {#each fields as field}
      <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">{field.label}</dt>
      <dd class="min-w-0 break-all text-phos">
        {#if field.type === "flag"}
          <span
            class="rounded-sm border border-gold/50 px-1.5 py-0.5 font-display text-[13px] uppercase text-gold"
          >
            {field.label}
          </span>
        {:else if field.type === "chips"}
          <span class="flex flex-wrap gap-1">
            {#each field.items as item}
              <span
                class="rounded-sm border border-line px-1.5 py-0.5 text-[11px] text-phos-dim"
              >{item}</span>
            {/each}
          </span>
        {:else if field.type === "link"}
          <a
            href={field.text}
            target="_blank"
            rel="noreferrer"
            class="text-accent hover:underline"
            onclick={(e) => {
              e.preventDefault();
              open(field.text);
            }}
          >{field.text}</a>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else if field.type === "path"}
          <span class="font-mono text-xs">{field.text}</span>
          <button type="button" class={iconBtn} onclick={() => reveal(field.text)}>📂 reveal</button>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else if field.type === "code"}
          <span class="font-mono text-xs">{field.text}</span>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else}
          {field.text}
        {/if}
      </dd>
    {/each}
  </dl>
{/if}
