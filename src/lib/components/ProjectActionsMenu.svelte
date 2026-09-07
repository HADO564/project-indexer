<script lang="ts">
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { refreshProjectTrackers } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass } from "./styles";

  // The standard row actions, extracted from ProjectCard unchanged. Every view
  // but the Bin uses this; the Bin supplies BinActions instead, because its
  // directory is gone and Open / Edit / Detect type make no sense there.
  let {
    project,
    onEdit,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onerror,
  }: {
    project: Project;
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

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") menuOpen = false;
  }}
/>
