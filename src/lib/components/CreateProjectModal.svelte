<script lang="ts">
  import type { Group } from "$lib/api/types";
  import CreateProjectForm from "./CreateProjectForm.svelte";
  import { buttonClass } from "./styles";

  // Creating a project is occasional; the project list is what you came for.
  // The form used to sit permanently above the list, and once it grew a group
  // select, eight swatches and a 24-glyph icon grid it pushed the list most of
  // the way off screen. Behind a button it costs nothing until you want it.
  let {
    groups,
    knownPropertyKeys = [],
    customIcons,
    onIconsChanged,
    onGroupsStale,
    onCreated,
    onClose,
    onerror,
  }: {
    groups: Group[];
    knownPropertyKeys?: string[];
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    onGroupsStale?: () => void | Promise<void>;
    onCreated: () => void | Promise<void>;
    onClose: () => void;
    onerror?: (message: string) => void;
  } = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  // Close on success. The form's fields reset by being destroyed with the
  // modal, so there is nothing to clear by hand.
  async function handleCreated() {
    await onCreated();
    onClose();
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
    aria-labelledby="create-project-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="mb-4 flex items-center justify-between">
      <h2 id="create-project-title" class="mt-0 text-lg font-semibold text-phos">
        <span class="text-gold">//</span> new project
      </h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>

    <CreateProjectForm
      {groups}
      {knownPropertyKeys}
      {customIcons}
      {onIconsChanged}
      {onGroupsStale}
      onCreated={handleCreated}
      {onerror}
    />
  </div>
</div>
