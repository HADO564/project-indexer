<script lang="ts">
  import { updateProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import AppPicker from "./AppPicker.svelte";
  import DirectoryField from "./DirectoryField.svelte";
  import { buttonClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    project,
    onSaved,
    onCancel,
    onerror,
  }: {
    project: Project;
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
  let client = $state(project.client ?? "");
  let openWith = $state(project.open_with ?? "");
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
        client: client || null,
        open_with: openWith || null,
      });
      await onSaved();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      saving = false;
    }
  }
</script>

<form
  onsubmit={handleSubmit}
  class="flex flex-col gap-3 rounded-md bg-gray-50 p-3 dark:bg-gray-900/40"
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
  <label class="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
    <input type="checkbox" bind:checked={favorite} />
    Favorite
  </label>
  <label class={labelClass}>
    Client
    <input bind:value={client} class={inputClass} />
  </label>
  <label class={labelClass}>
    Notes
    <input bind:value={notes} class={inputClass} />
  </label>
  <AppPicker bind:value={openWith} onerror={(m) => onerror?.(m)} />
  <div class="flex gap-2">
    <button type="submit" disabled={saving} class={primaryButtonClass}>
      {saving ? "Saving…" : "Save"}
    </button>
    <button type="button" onclick={onCancel} disabled={saving} class={buttonClass}>Cancel</button>
  </div>
</form>
