<script lang="ts">
  import type { Group } from "$lib/api/types";
  import { viewKey, type View, type ViewCounts } from "$lib/views";
  import SidebarEntry from "./SidebarEntry.svelte";

  // The rail. Groups are navigation, not sections: the current selection is
  // always visible and so is the fact that other groups exist, which is what
  // the rejected collapsible-band design could not do.
  //
  // It is a fixed column and does not collapse to an icon rail. Two groups may
  // legitimately pick the same glyph from the bundled set, and an icon-only
  // rail would then show two entries you cannot tell apart — the same "you
  // cannot see what is hidden" failure, in a smaller costume.
  let {
    groups,
    counts,
    selected,
    onSelect,
    showFavorites = false,
    showBin = false,
    onManageGroups,
  }: {
    groups: Group[];
    counts: ViewCounts;
    selected: View;
    onSelect: (view: View) => void;
    // Favourites and the Bin are still modals until their own tasks fold them
    // in, so each is verified against the modal it replaces.
    showFavorites?: boolean;
    showBin?: boolean;
    onManageGroups?: () => void;
  } = $props();

  const key = $derived(viewKey(selected));
</script>

<nav class="flex w-52 shrink-0 flex-col gap-1" aria-label="Views">
  <SidebarEntry
    icon="layers"
    label="All"
    count={counts.all}
    selected={key === "all"}
    onSelect={() => onSelect({ kind: "all" })}
  />
  {#if showFavorites}
    <SidebarEntry
      icon="star"
      label="Favourites"
      count={counts.favorites}
      color="gold"
      selected={key === "favorites"}
      onSelect={() => onSelect({ kind: "favorites" })}
    />
  {/if}

  <div class="mt-3 flex items-center justify-between px-2">
    <span class="font-display text-[12px] uppercase tracking-wide text-phos-faint">
      <span class="text-gold">//</span> groups
    </span>
    {#if onManageGroups}
      <button
        type="button"
        onclick={onManageGroups}
        class="rounded-sm px-1 font-display text-[14px] leading-none text-phos-faint hover:text-phos"
        title="Manage groups"
        aria-label="Manage groups"
      >
        +
      </button>
    {/if}
  </div>

  {#if groups.length === 0}
    <p class="px-2 text-[12px] text-phos-faint">No groups yet.</p>
  {:else}
    {#each groups as group (group.id)}
      <SidebarEntry
        icon={group.icon}
        label={group.name}
        count={counts.groups[group.id] ?? 0}
        color={group.color}
        selected={key === viewKey({ kind: "group", id: group.id })}
        onSelect={() => onSelect({ kind: "group", id: group.id })}
      />
    {/each}
  {/if}

  <SidebarEntry
    icon="box"
    label="Ungrouped"
    count={counts.ungrouped}
    selected={key === "ungrouped"}
    onSelect={() => onSelect({ kind: "ungrouped" })}
  />

  {#if showBin}
    <div class="mt-3">
      <SidebarEntry
        icon="trash"
        label="Bin"
        count={counts.bin}
        selected={key === "bin"}
        onSelect={() => onSelect({ kind: "bin" })}
      />
    </div>
  {/if}
</nav>
