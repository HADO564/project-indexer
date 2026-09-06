<script lang="ts">
  import { deleteProject, restoreProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass, dangerButtonClass } from "./styles";

  // The Bin's action set. Deliberately not the standard menu: a binned
  // project's directory is gone, so Open, Edit and Detect type make no sense
  // there. Taking actions as a snippet prop is what lets the same three row
  // layouts serve both sets without a second copy of the markup.
  let {
    project,
    // Purging is permanent, so the danger button asks for a second click
    // before it acts rather than stacking a confirmation dialog on top of the
    // view. A preserved invariant, not a detail.
    //
    // `armed` is owned by the list rather than by this row, which is what
    // BinModal did with its single `confirmPurgeId`: arming one row disarms
    // any other. Per-row state would let several rows sit armed at once, each
    // one click away from a permanent delete.
    armed,
    onArm,
    onRestored,
    onPurged,
    onerror,
  }: {
    project: Project;
    armed: boolean;
    onArm: () => void;
    onRestored: () => void | Promise<void>;
    onPurged: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let pending = $state(false);

  async function handleRestore() {
    pending = true;
    try {
      await restoreProject(project.id);
      await onRestored();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pending = false;
    }
  }

  async function handlePurge() {
    if (!armed) {
      onArm();
      return;
    }
    pending = true;
    try {
      await deleteProject(project.id);
      await onPurged();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pending = false;
    }
  }
</script>

<div class="flex shrink-0 gap-2">
  <button type="button" onclick={handleRestore} disabled={pending} class={buttonClass}>
    Restore
  </button>
  <button type="button" onclick={handlePurge} disabled={pending} class={dangerButtonClass}>
    {armed ? "Confirm?" : "Delete permanently"}
  </button>
</div>
