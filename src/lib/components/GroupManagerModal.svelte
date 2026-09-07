<script lang="ts">
  import { createGroup, deleteGroup, reorderGroups, updateGroup } from "$lib/api/groups";
  import type { Group } from "$lib/api/types";
  import { ICON_NAMES } from "$lib/icons";
  import { SWATCHES, isHexColor, swatchVar } from "$lib/palette";
  import BundledIcon from "./BundledIcon.svelte";
  import IconPicker from "./IconPicker.svelte";
  import SwatchPicker from "./SwatchPicker.svelte";
  import {
    buttonClass,
    dangerButtonClass,
    iconMd,
    inputClass,
    labelClass,
    primaryButtonClass,
  } from "./styles";

  let {
    groups,
    customIcons,
    onChanged,
    onClose,
    onerror,
  }: {
    groups: Group[];
    // Only so IconPicker can render; groups never take a custom icon, since
    // an <img> cannot be tinted to the group's colour.
    customIcons: Map<string, string>;
    onChanged: () => void | Promise<void>;
    onClose: () => void;
    onerror?: (message: string) => void;
  } = $props();

  let newName = $state("");
  let newColor = $state<string | null>("cyan");
  let newIcon = $state("briefcase");
  let busy = $state(false);
  // Deleting a group never deletes a project — its members become Ungrouped —
  // but it is still irreversible, so it takes a second click rather than a
  // dialog stacked on a dialog, matching the Bin's purge.
  let confirmDeleteId = $state<string | null>(null);
  // At most one group's icon grid is open at a time — 25 glyphs per group,
  // permanently expanded, made this dialog unreadable past two groups.
  let iconOpenFor = $state<string | null>(null);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  async function run(action: () => Promise<unknown>) {
    busy = true;
    try {
      await action();
      await onChanged();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      busy = false;
    }
  }

  async function handleCreate(event: Event) {
    event.preventDefault();
    const name = newName.trim();
    if (name.length === 0) return;
    await run(async () => {
      // A group must have a colour. The picker cannot produce null here
      // (allowNone is not set), but the bound type permits it.
      await createGroup(name, newColor ?? "cyan", newIcon);
      newName = "";
    });
  }

  // Renaming, recolouring and re-iconing all go through the same partial
  // update: a key left out means unchanged.
  async function handleRename(group: Group, name: string) {
    const trimmed = name.trim();
    if (trimmed.length === 0 || trimmed === group.name) return;
    await run(() => updateGroup(group.id, { name: trimmed }));
  }

  async function handleRecolour(group: Group, color: string) {
    if (color === group.color) return;
    await run(() => updateGroup(group.id, { color }));
  }

  async function handleReicon(group: Group, icon: string) {
    if (icon === group.icon) return;
    await run(() => updateGroup(group.id, { icon }));
  }

  // Reorder rewrites the whole ordering and returns what was persisted, so the
  // caller refetches rather than trusting its own optimistic order —
  // set_group_positions is the only thing that renumbers.
  async function handleMove(index: number, delta: number) {
    const next = index + delta;
    if (next < 0 || next >= groups.length) return;
    const ids = groups.map((g) => g.id);
    [ids[index], ids[next]] = [ids[next], ids[index]];
    await run(() => reorderGroups(ids));
  }

  async function handleDelete(group: Group) {
    if (confirmDeleteId !== group.id) {
      confirmDeleteId = group.id;
      return;
    }
    await run(async () => {
      await deleteGroup(group.id);
      confirmDeleteId = null;
    });
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
    class="max-h-[85vh] w-11/12 max-w-xl overflow-y-auto rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="group-manager-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="flex items-center justify-between">
      <h2 id="group-manager-title" class="mt-0 text-lg font-semibold text-phos">Groups</h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>
    <p class="mt-2 text-sm text-phos-dim">
      A project belongs to one group at a time. Deleting a group never deletes a project — its
      members become Ungrouped.
    </p>

    <form onsubmit={handleCreate} class="mt-4 flex flex-col gap-3 rounded-sm bg-panel-2 p-3">
      <label class={labelClass}>
        New group
        <input bind:value={newName} placeholder="Client work" required class={inputClass} />
      </label>
      <SwatchPicker bind:value={newColor} allowCustom />
      <IconPicker bind:value={newIcon} {customIcons} bundledOnly allowNone={false} />
      <button type="submit" disabled={busy} class={`self-start ${primaryButtonClass}`}>
        {busy ? "Working…" : "Add group"}
      </button>
    </form>

    {#if groups.length === 0}
      <p class="mt-4 text-sm text-phos-dim">No groups yet.</p>
    {:else}
      <ul class="mt-4 flex flex-col gap-2">
        {#each groups as group, index (group.id)}
          <li class="rounded-sm border border-line p-3">
            <div class="flex flex-wrap items-center gap-2">
              <span class="shrink-0" style={`color: ${swatchVar(group.color)}`}>
                <BundledIcon name={group.icon} class={iconMd} />
              </span>
              <input
                value={group.name}
                disabled={busy}
                onchange={(e) => handleRename(group, e.currentTarget.value)}
                class={`min-w-0 flex-1 ${inputClass}`}
                aria-label={`Rename ${group.name}`}
              />
              <button
                type="button"
                disabled={busy || index === 0}
                onclick={() => handleMove(index, -1)}
                class={buttonClass}
                aria-label={`Move ${group.name} up`}
              >
                ↑
              </button>
              <button
                type="button"
                disabled={busy || index === groups.length - 1}
                onclick={() => handleMove(index, 1)}
                class={buttonClass}
                aria-label={`Move ${group.name} down`}
              >
                ↓
              </button>
              <button
                type="button"
                disabled={busy}
                onclick={() => handleDelete(group)}
                class={dangerButtonClass}
              >
                {confirmDeleteId === group.id ? "Confirm?" : "Delete"}
              </button>
            </div>

            <!-- SwatchPicker binds a value; recolouring an existing group needs
                 a call instead, so these two rows commit directly. -->
            <div class="mt-2 flex flex-wrap items-center gap-1.5">
              {#each SWATCHES as swatch}
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => handleRecolour(group, swatch)}
                  title={swatch}
                  aria-label={`${swatch} for ${group.name}`}
                  aria-pressed={group.color === swatch}
                  class={`h-7 w-7 rounded-full border-2 ${
                    group.color === swatch ? "border-phos" : "border-transparent"
                  }`}
                  style={`background: ${swatchVar(swatch)}`}
                ></button>
              {/each}

              <!-- onchange, not oninput: a colour input fires continuously
                   while the picker is dragged, and every change here is a
                   backend write. onchange fires once, on commit. -->
              <label
                class="relative inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-full border-2"
                style={`border-color: ${isHexColor(group.color) ? "var(--color-phos)" : "transparent"}; background: ${swatchVar(group.color)}`}
                title="Custom colour"
              >
                <input
                  type="color"
                  value={isHexColor(group.color) ? group.color : "#8899aa"}
                  disabled={busy}
                  onchange={(e) => handleRecolour(group, e.currentTarget.value)}
                  class="absolute inset-0 cursor-pointer opacity-0"
                  aria-label={`Custom colour for ${group.name}`}
                />
                <span
                  class="pointer-events-none font-display text-[12px] leading-none text-void mix-blend-difference"
                  aria-hidden="true">+</span
                >
              </label>
            </div>
            <button
              type="button"
              onclick={() => (iconOpenFor = iconOpenFor === group.id ? null : group.id)}
              class={`mt-2 ${buttonClass}`}
              aria-expanded={iconOpenFor === group.id}
            >
              {iconOpenFor === group.id ? "Done" : "Change icon"}
            </button>

            {#if iconOpenFor === group.id}
              <div class="mt-1 flex flex-wrap items-center gap-1 rounded-sm border border-line p-2">
                {#each ICON_NAMES as name}
                  <button
                    type="button"
                    disabled={busy}
                    onclick={() => {
                      handleReicon(group, name);
                      iconOpenFor = null;
                    }}
                    title={name}
                    aria-label={`${name} icon for ${group.name}`}
                    aria-pressed={group.icon === name}
                    class={`inline-flex h-10 w-10 items-center justify-center rounded-sm border-2 ${
                      group.icon === name
                        ? "border-phos text-phos"
                        : "border-transparent text-phos-dim hover:text-phos"
                    }`}
                  >
                    <BundledIcon {name} class={iconMd} />
                  </button>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
