<script lang="ts">
  import type { Group, Project } from "$lib/api/types";
  import EditProjectForm from "./EditProjectForm.svelte";
  import { buttonClass } from "./styles";

  // Editing used to swap a list row in place, which reflowed the whole list
  // around a form taller than the row it replaced — and grew worse once the
  // form gained a group select, eight swatches and a 24-glyph icon grid. A
  // dialog keeps the list still behind it.
  //
  // /project/[id] had already hand-rolled its own edit dialog; it uses this
  // one now, so there is a single edit surface rather than two that drift.
  let {
    project,
    groups,
    knownPropertyKeys = [],
    customIcons,
    onIconsChanged,
    onGroupsStale,
    onSaved,
    onClose,
    onerror,
  }: {
    project: Project;
    groups: Group[];
    knownPropertyKeys?: string[];
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    onGroupsStale?: () => void | Promise<void>;
    // Both callers already clear their own editing state here, so this does
    // not close itself — that would be a double close.
    onSaved: () => void | Promise<void>;
    onClose: () => void;
    onerror?: (message: string) => void;
  } = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-void/85 p-4"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class="max-h-[85vh] w-11/12 max-w-xl overflow-y-auto rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="edit-project-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="mb-4 flex items-center gap-3">
      <h2 id="edit-project-title" class="mt-0 min-w-0 flex-1 truncate text-lg font-semibold text-phos">
        <span class="text-accent">&gt;</span>&nbsp;edit {project.name}
      </h2>
      <button type="button" onclick={onClose} class={`shrink-0 ${buttonClass}`}>Close</button>
    </div>

    <EditProjectForm
      {project}
      {groups}
      {customIcons}
      {onIconsChanged}
      {onGroupsStale}
      {onSaved}
      onCancel={onClose}
      {onerror}
    />
  </div>
</div>
