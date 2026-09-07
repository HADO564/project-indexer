<script lang="ts">
  import { isGroupNotFound } from "$lib/api/groups";
  import { createProject, suggestProjectName, updateProject } from "$lib/api/projects";
  import type { CreateProjectInput, Group } from "$lib/api/types";
  import DirectoryField from "./DirectoryField.svelte";
  import IconPicker from "./IconPicker.svelte";
  import SwatchPicker from "./SwatchPicker.svelte";
  import { inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    groups,
    customIcons,
    onIconsChanged,
    onGroupsStale,
    onCreated,
    onerror,
  }: {
    groups: Group[];
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    onGroupsStale?: () => void | Promise<void>;
    onCreated: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let name = $state("");
  let directory = $state("");
  let description = $state("");
  let tags = $state("");
  let color = $state<string | null>(null);
  let icon = $state<string | null>(null);
  let groupId = $state<string | null>(null);
  let creating = $state(false);

  function parseTags(raw: string): string[] {
    return raw
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }


  async function handleDirectoryPicked(dir: string) {
    if (name.trim().length > 0) return;
    try {
      const suggested = await suggestProjectName(dir);
      if (name.trim().length > 0) return; // user may have typed while we awaited
      if (suggested) name = suggested;
    } catch {
      // No suggestion — the user types a name manually.
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    creating = true;
    try {
      const input: CreateProjectInput = {
        name,
        directory,
        description: description || null,
        tags: parseTags(tags),
      };
      const created = await createProject(input);
      // create_project takes only name/directory/description/tags, so the
      // three new fields are applied straight after. One extra call against a
      // local database, and CreateProjectInput stays unchanged.
      if (color || icon || groupId) {
        await updateProject(created.id, { color, icon, group_id: groupId });
      }
      name = "";
      directory = "";
      description = "";
      tags = "";
      color = null;
      icon = null;
      groupId = null;
      await onCreated();
    } catch (err) {
      // The project itself was created; only the group assignment failed,
      // because the group was deleted between opening this form and
      // submitting it. Clear the stale selection and refetch rather than
      // retrying — the backend rejected the write and changed nothing.
      if (isGroupNotFound(err)) {
        groupId = null;
        await onGroupsStale?.();
        onerror?.("That group no longer exists — the project was created without one.");
        await onCreated();
      } else {
        onerror?.((err as Error).message);
      }
    } finally {
      creating = false;
    }
  }
</script>

<form onsubmit={handleSubmit} class="flex flex-col gap-3">
  <label class={labelClass}>
    Name
    <input bind:value={name} required placeholder="My project" class={inputClass} />
  </label>
  <DirectoryField
    bind:value={directory}
    required
    onerror={(m) => onerror?.(m)}
    onPicked={handleDirectoryPicked}
  />
  <label class={labelClass}>
    Description
    <input bind:value={description} placeholder="Optional description" class={inputClass} />
  </label>
  <label class={labelClass}>
    Tags (comma separated)
    <input bind:value={tags} placeholder="rust, tauri" class={inputClass} />
  </label>
  <label class={labelClass}>
    Group
    <select bind:value={groupId} class={inputClass}>
      <option value={null}>Ungrouped</option>
      {#each groups as group (group.id)}
        <option value={group.id}>{group.name}</option>
      {/each}
    </select>
  </label>
  <SwatchPicker bind:value={color} allowNone />
  <IconPicker bind:value={icon} {customIcons} {onIconsChanged} onerror={(m) => onerror?.(m)} />
  <button type="submit" disabled={creating} class={`self-start ${primaryButtonClass}`}>
    {creating ? "Creating…" : "Create project"}
  </button>
</form>
