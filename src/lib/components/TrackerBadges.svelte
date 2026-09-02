<script lang="ts">
  import type { Tracker } from "$lib/api/types";
  import { trackerColor, trackerKind } from "$lib/trackers";

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
      {@const c = trackerColor(trackerKind(tracker))}
      <span
        class="inline-flex items-center gap-1 rounded-sm border px-1.5 py-0.5 font-display text-[13px]"
        style="color: {c}; border-color: color-mix(in srgb, {c} 60%, transparent)"
      >
        {trackerKind(tracker)}
        {#if detail(tracker)}
          <span class="opacity-70">· {detail(tracker)}</span>
        {/if}
        {#if isDirty(tracker)}
          <span class="text-gold" title="Uncommitted changes">●</span>
        {/if}
      </span>
    {/each}
  </div>
{/if}
