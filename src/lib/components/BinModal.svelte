<script lang="ts">
  import { deleteProject, getDeletedProjects, restoreProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass, dangerButtonClass } from "./styles";

  let {
    onClose,
    onRestored,
    onerror,
  }: {
    onClose: () => void;
    onRestored: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let projects = $state<Project[]>([]);
  let loading = $state(false);
  let pendingId = $state<string | null>(null);
  // Purging is permanent, so the danger button asks for a second click
  // before it acts rather than opening yet another modal on top of this one.
  let confirmPurgeId = $state<string | null>(null);

  async function load() {
    loading = true;
    try {
      projects = await getDeletedProjects();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  async function handleRestore(project: Project) {
    pendingId = project.id;
    try {
      await restoreProject(project.id);
      projects = projects.filter((p) => p.id !== project.id);
      await onRestored();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pendingId = null;
    }
  }

  async function handlePurge(project: Project) {
    if (confirmPurgeId !== project.id) {
      confirmPurgeId = project.id;
      return;
    }
    pendingId = project.id;
    try {
      await deleteProject(project.id);
      projects = projects.filter((p) => p.id !== project.id);
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pendingId = null;
      confirmPurgeId = null;
    }
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
    class="w-11/12 max-w-lg rounded-lg bg-white p-6 shadow-2xl dark:bg-gray-800"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="bin-modal-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="flex items-center justify-between">
      <h2 id="bin-modal-title" class="mt-0 text-lg font-semibold text-gray-900 dark:text-gray-100">
        Bin
      </h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>
    <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
      Projects whose directory was deleted, kept here until restored or forgotten for good.
    </p>

    {#if loading}
      <p class="mt-4 text-sm text-gray-500 dark:text-gray-400">Loading…</p>
    {:else if projects.length === 0}
      <p class="mt-4 text-sm text-gray-500 dark:text-gray-400">The bin is empty.</p>
    {:else}
      <ul class="mt-4 flex max-h-80 flex-col gap-2 overflow-y-auto">
        {#each projects as project (project.id)}
          <li class="rounded-md border border-gray-200 p-3 dark:border-gray-700">
            <div class="flex items-start justify-between gap-4">
              <div>
                <strong class="text-gray-900 dark:text-gray-100">{project.name}</strong>
                <div class="text-sm break-all text-gray-500 dark:text-gray-400">
                  {project.directory}
                </div>
              </div>
              <div class="flex shrink-0 gap-2">
                <button
                  type="button"
                  onclick={() => handleRestore(project)}
                  disabled={pendingId === project.id}
                  class={buttonClass}
                >
                  Restore
                </button>
                <button
                  type="button"
                  onclick={() => handlePurge(project)}
                  disabled={pendingId === project.id}
                  class={dangerButtonClass}
                >
                  {confirmPurgeId === project.id ? "Confirm?" : "Delete permanently"}
                </button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
