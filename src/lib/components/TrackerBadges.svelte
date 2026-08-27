<script lang="ts">
  import type { Tracker } from "$lib/api/types";
  import { trackerKind } from "$lib/trackers";

  let { trackers }: { trackers: Tracker[] } = $props();

  // Secondary detail shown after the label (branch for Git, VCS provider
  // for Unreal) — the two trackers don't share a concept here, so this
  // isn't quite "branch," but it plays the same role in the badge.
  function detail(tracker: Tracker): string | null {
    if (typeof tracker === "string") return null;
    if ("Git" in tracker) return tracker.Git.curr_branch;
    return tracker.Unreal.vcs_provider;
  }

  function isDirty(tracker: Tracker): boolean {
    return typeof tracker !== "string" && "Git" in tracker && tracker.Git.dirty;
  }
</script>

{#if trackers.length > 0}
  <div class="mt-2 flex flex-wrap gap-1.5">
    {#each trackers as tracker}
      <span
        class="inline-flex items-center gap-1 rounded-full bg-gray-100 px-2.5 py-0.5 text-xs text-gray-700 dark:bg-gray-700 dark:text-gray-300"
      >
        {trackerKind(tracker)}
        {#if detail(tracker)}
          <span class="text-gray-400 dark:text-gray-500">· {detail(tracker)}</span>
        {/if}
        {#if isDirty(tracker)}
          <span class="text-amber-500" title="Uncommitted changes">●</span>
        {/if}
      </span>
    {/each}
  </div>
{/if}
