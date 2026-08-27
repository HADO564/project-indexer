<script lang="ts">
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { refreshProjectTrackers } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass } from "./styles";
  import TrackerBadges from "./TrackerBadges.svelte";

  let {
    project,
    onEdit,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onShowDetails,
    onerror,
  }: {
    project: Project;
    onEdit: (project: Project) => void;
    onRequestDelete: (project: Project) => void;
    onOpened: () => void | Promise<void>;
    onTrackersRefreshed: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    onShowDetails: (project: Project) => void;
    onerror?: (message: string) => void;
  } = $props();

  let refreshing = $state(false);

  async function handleOpen() {
    try {
      await openProjectDirectory(project.id);
      await onOpened();
    } catch (err) {
      if (isOpenWithAppMissing(err)) {
        onOpenWithAppMissing(project);
      } else {
        onerror?.((err as Error).message);
      }
    }
  }

  async function handleRefreshTrackers() {
    refreshing = true;
    try {
      await refreshProjectTrackers(project.id);
      await onTrackersRefreshed();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      refreshing = false;
    }
  }
</script>

<div class="flex items-start justify-between gap-4">
  <div>
    <div class="flex items-center gap-1">
      <strong class="text-gray-900 dark:text-gray-100">{project.name}</strong>
      {#if project.favorite}<span title="Favorite">★</span>{/if}
    </div>
    <div class="text-sm text-gray-500 dark:text-gray-400">{project.directory}</div>
    {#if project.description}
      <p class="mt-1 text-sm text-gray-700 dark:text-gray-300">{project.description}</p>
    {/if}
    {#if project.tags.length > 0}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each project.tags as tag}
          <span
            class="rounded-full bg-indigo-100 px-2.5 py-0.5 text-xs text-indigo-800 dark:bg-indigo-950 dark:text-indigo-300"
          >
            {tag}
          </span>
        {/each}
      </div>
    {/if}
    <TrackerBadges trackers={project.trackers} />
  </div>
  <div class="flex shrink-0 gap-2">
    <button type="button" onclick={handleOpen} class={buttonClass}>Open</button>
    <button type="button" onclick={() => onShowDetails(project)} class={buttonClass}>
      Details
    </button>
    <button
      type="button"
      onclick={handleRefreshTrackers}
      disabled={refreshing}
      class={buttonClass}
      title="Re-detect project type (git, ...)"
    >
      {refreshing ? "Detecting…" : "Detect"}
    </button>
    <button type="button" onclick={() => onEdit(project)} class={buttonClass}>Edit</button>
    <button type="button" onclick={() => onRequestDelete(project)} class={buttonClass}>
      Delete
    </button>
  </div>
</div>
