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
      onerror?.((err as Error).message);
    }
  }

  async function reveal(path: string) {
    try {
      await revealPath(path);
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }

  const iconBtn =
    "text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 text-xs";
</script>

{#if fields.length === 0}
  <p class="text-sm text-gray-500 dark:text-gray-400">No details available.</p>
{:else}
  <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-2 text-sm">
    {#each fields as field}
      <dt class="text-gray-500 dark:text-gray-400">{field.label}</dt>
      <dd class="min-w-0 break-all text-gray-900 dark:text-gray-100">
        {#if field.type === "flag"}
          <span
            class="rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-800 dark:bg-amber-950 dark:text-amber-300"
          >
            {field.label}
          </span>
        {:else if field.type === "chips"}
          <span class="flex flex-wrap gap-1">
            {#each field.items as item}
              <span
                class="rounded bg-gray-100 px-1.5 py-0.5 text-xs dark:bg-gray-700"
              >{item}</span>
            {/each}
          </span>
        {:else if field.type === "link"}
          <a
            href={field.text}
            target="_blank"
            rel="noreferrer"
            class="text-blue-600 hover:underline dark:text-blue-400"
          >{field.text}</a>
          <button type="button" class={iconBtn} onclick={() => open(field.text)}>↗ open</button>
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
