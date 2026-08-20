<script lang="ts">
  import { createProject } from "$lib/api/projects";
  import type { CreateProjectInput } from "$lib/api/types";
  import DirectoryField from "./DirectoryField.svelte";
  import { cardClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    onCreated,
    onerror,
  }: {
    onCreated: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let name = $state("");
  let directory = $state("");
  let description = $state("");
  let tags = $state("");
  let creating = $state(false);

  function parseTags(raw: string): string[] {
    return raw
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
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
      await createProject(input);
      name = "";
      directory = "";
      description = "";
      tags = "";
      await onCreated();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      creating = false;
    }
  }
</script>

<section class={`mb-6 ${cardClass}`}>
  <h2 class="mb-3 text-lg font-semibold text-gray-900 dark:text-gray-100">New project</h2>
  <form onsubmit={handleSubmit} class="flex flex-col gap-3">
    <label class={labelClass}>
      Name
      <input bind:value={name} required placeholder="My project" class={inputClass} />
    </label>
    <DirectoryField bind:value={directory} required onerror={(m) => onerror?.(m)} />
    <label class={labelClass}>
      Description
      <input bind:value={description} placeholder="Optional description" class={inputClass} />
    </label>
    <label class={labelClass}>
      Tags (comma separated)
      <input bind:value={tags} placeholder="rust, tauri" class={inputClass} />
    </label>
    <button type="submit" disabled={creating} class={`self-start ${primaryButtonClass}`}>
      {creating ? "Creating…" : "Create project"}
    </button>
  </form>
</section>
