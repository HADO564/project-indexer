<script lang="ts">
  import { untrack } from "svelte";
  import { isGroupNotFound } from "$lib/api/groups";
  import { updateProject } from "$lib/api/projects";
  import type { Group, Project } from "$lib/api/types";
  import AppPicker from "./AppPicker.svelte";
  import DirectoryField from "./DirectoryField.svelte";
  import IconPicker from "./IconPicker.svelte";
  import PropertyEditor from "./PropertyEditor.svelte";
  import SwatchPicker from "./SwatchPicker.svelte";
  import { buttonClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    project,
    groups,
    knownPropertyKeys = [],
    customIcons,
    onIconsChanged,
    onGroupsStale,
    onSaved,
    onCancel,
    onerror,
  }: {
    project: Project;
    groups: Group[];
    // Property names already used elsewhere, so a second project reuses
    // "client" rather than inventing "Client" beside it.
    knownPropertyKeys?: string[];
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    // Called when the backend rejects the write because the selected group no
    // longer exists, so the caller can refetch the group list.
    onGroupsStale?: () => void | Promise<void>;
    onSaved: () => void | Promise<void>;
    onCancel: () => void;
    onerror?: (message: string) => void;
  } = $props();

  let name = $state(project.name);
  let directory = $state(project.directory);
  let description = $state(project.description);
  let tags = $state(project.tags.join(", "));
  let favorite = $state(project.favorite);
  let notes = $state(project.notes ?? "");
  let properties = $state<Record<string, string>>(untrack(() => ({ ...project.properties })));
  let openWith = $state(project.open_with ?? "");
  // Read through untrack: these capture the initial value on purpose, exactly
  // as the eight above do, but saying so explicitly keeps svelte-check's
  // state_referenced_locally count at the documented PI-003 baseline of 8
  // rather than adding three more of the same false positive.
  let color = $state<string | null>(untrack(() => project.color));
  let icon = $state<string | null>(untrack(() => project.icon));
  let groupId = $state<string | null>(untrack(() => project.group_id));
  let saving = $state(false);

  function parseTags(raw: string): string[] {
    return raw
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    saving = true;
    try {
      await updateProject(project.id, {
        name,
        directory,
        description,
        tags: parseTags(tags),
        favorite,
        notes: notes || null,
        properties,
        open_with: openWith || null,
        group_id: groupId,
        color,
        icon,
      });
      await onSaved();
    } catch (err) {
      // The group can be deleted from the group manager while this form is
      // open. The backend rejects the write and changes nothing, so refetch
      // the group list and clear the stale selection — retrying is guaranteed
      // to fail the same way. Clearing a group (group_id: null) is always
      // allowed and checks nothing.
      if (isGroupNotFound(err)) {
        groupId = null;
        await onGroupsStale?.();
        onerror?.("That group no longer exists — the selection was cleared. Save again.");
      } else {
        onerror?.((err as Error).message);
      }
    } finally {
      saving = false;
    }
  }
</script>

<form
  onsubmit={handleSubmit}
  class="flex flex-col gap-3 rounded-md bg-panel-2 p-3"
>
  <label class={labelClass}>
    Name
    <input bind:value={name} required class={inputClass} />
  </label>
  <DirectoryField bind:value={directory} required onerror={(m) => onerror?.(m)} />
  <label class={labelClass}>
    Description
    <input bind:value={description} class={inputClass} />
  </label>
  <label class={labelClass}>
    Tags (comma separated)
    <input bind:value={tags} class={inputClass} />
  </label>
  <label class="flex items-center gap-2 text-sm text-phos-dim">
    <input type="checkbox" bind:checked={favorite} />
    Favorite
  </label>
  <label class={labelClass}>
    Notes
    <input bind:value={notes} class={inputClass} />
  </label>
  <AppPicker bind:value={openWith} onerror={(m) => onerror?.(m)} />
  <label class={labelClass}>
    Group
    <select bind:value={groupId} class={inputClass}>
      <option value={null}>Ungrouped</option>
      {#each groups as group (group.id)}
        <option value={group.id}>{group.name}</option>
      {/each}
    </select>
  </label>
  <SwatchPicker bind:value={color} allowNone allowCustom />
  <IconPicker bind:value={icon} {customIcons} {onIconsChanged} onerror={(m) => onerror?.(m)} />
  <PropertyEditor bind:value={properties} knownKeys={knownPropertyKeys} />
  <div class="flex gap-2">
    <button type="submit" disabled={saving} class={primaryButtonClass}>
      {saving ? "Saving…" : "Save"}
    </button>
    <button type="button" onclick={onCancel} disabled={saving} class={buttonClass}>Cancel</button>
  </div>
</form>
