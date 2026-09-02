<script lang="ts">
  import type { Project } from "$lib/api/types";

  let { project }: { project: Project } = $props();

  function formatDate(iso: string | null): string {
    return iso ? new Date(iso).toLocaleString() : "—";
  }
</script>

<dl class="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-sm">
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
