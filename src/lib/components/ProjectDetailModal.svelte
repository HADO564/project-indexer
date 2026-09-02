<script lang="ts">
  import type { Project } from "$lib/api/types";
  import { trackerKind } from "$lib/trackers";
  import { cardClass } from "./styles";
  import TrackerPanel from "./TrackerPanel.svelte";

  let {
    project,
    onClose,
  }: {
    project: Project;
    onClose: () => void;
  } = $props();

  // One tab per detected tracker (git, unreal, ...) — a new detector shows
  // up here automatically, no changes needed in this file.
  let activeIndex = $state(0);

  // Keeps the active tab in range if the tracker list shrinks (e.g. a
  // re-detect drops one) while the modal is open.
  $effect(() => {
    if (activeIndex >= project.trackers.length) activeIndex = 0;
  });

  function formatDate(iso: string | null): string {
    if (!iso) return "—";
    return new Date(iso).toLocaleString();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class={`w-11/12 max-w-lg ${cardClass} max-h-[85vh] overflow-y-auto shadow-2xl`}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="project-detail-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="flex items-start justify-between gap-4">
      <h2 id="project-detail-title" class="mt-0 text-lg font-semibold text-gray-900 dark:text-gray-100">
        {project.name}
      </h2>
      <button
        type="button"
        onclick={onClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
        aria-label="Close"
      >
        ✕
      </button>
    </div>

    <dl class="mt-3 grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-sm">
      <dt class="text-gray-500 dark:text-gray-400">Directory</dt>
      <dd class="break-all text-gray-900 dark:text-gray-100">{project.directory}</dd>

      {#if project.description}
        <dt class="text-gray-500 dark:text-gray-400">Description</dt>
        <dd class="text-gray-900 dark:text-gray-100">{project.description}</dd>
      {/if}

      {#if project.client}
        <dt class="text-gray-500 dark:text-gray-400">Client</dt>
        <dd class="text-gray-900 dark:text-gray-100">{project.client}</dd>
      {/if}

      {#if project.tags.length > 0}
        <dt class="text-gray-500 dark:text-gray-400">Tags</dt>
        <dd class="text-gray-900 dark:text-gray-100">{project.tags.join(", ")}</dd>
      {/if}

      {#if project.notes}
        <dt class="text-gray-500 dark:text-gray-400">Notes</dt>
        <dd class="text-gray-900 dark:text-gray-100">{project.notes}</dd>
      {/if}

      <dt class="text-gray-500 dark:text-gray-400">Created</dt>
      <dd class="text-gray-900 dark:text-gray-100">{formatDate(project.created_at)}</dd>

      <dt class="text-gray-500 dark:text-gray-400">Last opened</dt>
      <dd class="text-gray-900 dark:text-gray-100">{formatDate(project.last_opened_at)}</dd>
    </dl>

    <h3 class="mt-4 text-sm font-semibold text-gray-900 dark:text-gray-100">Trackers</h3>

    {#if project.trackers.length === 0}
      <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
        No trackers detected yet — try "Detect" on the project card.
      </p>
    {:else}
      <div class="mt-2 flex flex-wrap gap-1 border-b border-gray-200 dark:border-gray-700">
        {#each project.trackers as tracker, i}
          <button
            type="button"
            role="tab"
            aria-selected={activeIndex === i}
            onclick={() => (activeIndex = i)}
            class={`rounded-t-md px-3 py-1.5 text-sm font-medium ${
              activeIndex === i
                ? "bg-gray-100 text-gray-900 dark:bg-gray-700 dark:text-gray-100"
                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
            }`}
          >
            {trackerKind(tracker)}
          </button>
        {/each}
      </div>

      {#each project.trackers as tracker, i}
        {#if activeIndex === i}
          <div role="tabpanel" class="p-3">
            <TrackerPanel {tracker} />
          </div>
        {/if}
      {/each}
    {/if}
  </div>
</div>
