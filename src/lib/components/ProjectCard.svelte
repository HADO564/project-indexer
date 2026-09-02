<script lang="ts">
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { refreshProjectTrackers } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass } from "./styles";
  import TrackerBadges from "./TrackerBadges.svelte";

  let {
    project,
    directoryMissing = false,
    onEdit,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onerror,
  }: {
    project: Project;
    directoryMissing?: boolean;
    onEdit: (project: Project) => void;
    onRequestDelete: (project: Project) => void;
    onOpened: () => void | Promise<void>;
    onTrackersRefreshed: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    onerror?: (message: string) => void;
  } = $props();

  let refreshing = $state(false);
  let menuOpen = $state(false);

  const menuItem =
    "px-3 py-1.5 text-left font-display text-[14px] text-phos-dim hover:bg-panel-2 hover:text-phos disabled:cursor-default disabled:text-phos-faint disabled:hover:bg-transparent disabled:hover:text-phos-faint";

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

<div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
  <div class="min-w-0 flex-1">
    <div class="flex min-w-0 items-center gap-2">
      <strong class="min-w-0 truncate font-display text-[15px] text-phos">
        <span class="text-accent">&gt;</span>&nbsp;{project.name}
      </strong>
      {#if project.favorite}<span class="shrink-0 text-gold" title="Favorite">★</span>{/if}
      {#if directoryMissing}
        <span class="shrink-0 text-amber" title="Directory deleted or moved">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-4 w-4"
          >
            <path d="M3 6h18" />
            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6" />
            <path d="M14 11v6" />
          </svg>
        </span>
      {/if}
    </div>
    <div
      class={`truncate text-[12px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    >
      {project.directory}
    </div>
    {#if project.description}
      <p class="mt-1 text-sm text-phos-dim">{project.description}</p>
    {/if}
    {#if project.tags.length > 0}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each project.tags as tag}
          <span
            class="rounded-sm border border-line px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-phos-dim"
          >
            {tag}
          </span>
        {/each}
      </div>
    {/if}
    <TrackerBadges trackers={project.trackers} />
  </div>
  <div class="relative shrink-0">
    <button
      type="button"
      onclick={() => (menuOpen = !menuOpen)}
      class={buttonClass}
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      aria-label="Project actions"
    >
      ···
    </button>

    {#if menuOpen}
      <button
        type="button"
        class="fixed inset-0 z-10 cursor-default"
        tabindex="-1"
        aria-label="Close menu"
        onclick={() => (menuOpen = false)}
      ></button>
      <div
        role="menu"
        class="absolute right-0 z-20 mt-1 flex min-w-40 flex-col rounded-sm border border-line bg-panel py-1"
      >
        <button
          role="menuitem"
          type="button"
          class={menuItem}
          onclick={() => {
            menuOpen = false;
            handleOpen();
          }}
        >
          Open
        </button>
        <a role="menuitem" href={`/project/${project.id}`} class={menuItem} onclick={() => (menuOpen = false)}>
          Details
        </a>
        <button
          role="menuitem"
          type="button"
          class={menuItem}
          disabled={refreshing}
          onclick={() => {
            menuOpen = false;
            handleRefreshTrackers();
          }}
        >
          {refreshing ? "Detecting…" : "Detect type"}
        </button>
        <button
          role="menuitem"
          type="button"
          class={menuItem}
          onclick={() => {
            menuOpen = false;
            onEdit(project);
          }}
        >
          Edit
        </button>
        <button
          role="menuitem"
          type="button"
          class={`${menuItem} hover:bg-rust! hover:text-void!`}
          onclick={() => {
            menuOpen = false;
            onRequestDelete(project);
          }}
        >
          Delete
        </button>
      </div>
    {/if}
  </div>
</div>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") menuOpen = false;
  }}
/>
