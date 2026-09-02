<script lang="ts">
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { getFavoriteProjects, updateProject } from "$lib/api/projects";
  import type { Project, SortBy, SortDirection } from "$lib/api/types";
  import SortControls from "./SortControls.svelte";
  import { buttonClass } from "./styles";

  let {
    onClose,
    onChanged,
    onOpenWithAppMissing,
    onerror,
  }: {
    onClose: () => void;
    onChanged: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    onerror?: (message: string) => void;
  } = $props();

  let projects = $state<Project[]>([]);
  let loading = $state(false);
  let pendingId = $state<string | null>(null);
  let sortBy = $state<SortBy>("alphabetical");
  let sortDirection = $state<SortDirection>("ascending");

  async function load() {
    loading = true;
    try {
      projects = await getFavoriteProjects({ by: sortBy, direction: sortDirection });
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

  async function handleOpen(project: Project) {
    pendingId = project.id;
    try {
      await openProjectDirectory(project.id);
      await onChanged();
    } catch (err) {
      if (isOpenWithAppMissing(err)) {
        onOpenWithAppMissing(project);
      } else {
        onerror?.((err as Error).message);
      }
    } finally {
      pendingId = null;
    }
  }

  async function handleUnfavorite(project: Project) {
    pendingId = project.id;
    try {
      await updateProject(project.id, { favorite: false });
      projects = projects.filter((p) => p.id !== project.id);
      await onChanged();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pendingId = null;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-void/85"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class="w-11/12 max-w-lg rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="favorites-modal-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="flex items-center justify-between">
      <h2
        id="favorites-modal-title"
        class="mt-0 text-lg font-semibold text-phos"
      >
        Favorites
      </h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>

    <div class="mt-3">
      <SortControls bind:by={sortBy} bind:direction={sortDirection} />
    </div>

    {#if loading}
      <p class="mt-4 text-sm text-phos-dim">Loading…</p>
    {:else if projects.length === 0}
      <p class="mt-4 text-sm text-phos-dim">No favorites yet.</p>
    {:else}
      <ul class="mt-4 flex max-h-80 flex-col gap-2 overflow-y-auto">
        {#each projects as project (project.id)}
          <li class="rounded-sm border border-line p-3">
            <div class="flex items-start justify-between gap-4">
              <div>
                <strong class="text-phos">{project.name}</strong>
                <div class="text-sm break-all text-phos-dim">
                  {project.directory}
                </div>
              </div>
              <div class="flex shrink-0 gap-2">
                <button
                  type="button"
                  onclick={() => handleOpen(project)}
                  disabled={pendingId === project.id}
                  class={buttonClass}
                >
                  Open
                </button>
                <button
                  type="button"
                  onclick={() => handleUnfavorite(project)}
                  disabled={pendingId === project.id}
                  class={buttonClass}
                  title="Remove from favorites"
                >
                  ★
                </button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
