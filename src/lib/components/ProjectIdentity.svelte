<script lang="ts">
  import type { Project } from "$lib/api/types";

  let { project }: { project: Project } = $props();

  function formatDate(iso: string | null): string {
    return iso ? new Date(iso).toLocaleString() : "—";
  }
</script>

<dl class="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-sm">
  <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Directory</dt>
  <dd class="break-all text-phos">{project.directory}</dd>

  {#if project.description}
    <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Description</dt>
    <dd class="text-phos">{project.description}</dd>
  {/if}
  {#if project.client}
    <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Client</dt>
    <dd class="text-phos">{project.client}</dd>
  {/if}
  {#if project.tags.length > 0}
    <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Tags</dt>
    <dd class="text-phos">{project.tags.join(", ")}</dd>
  {/if}
  {#if project.notes}
    <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Notes</dt>
    <dd class="text-phos">{project.notes}</dd>
  {/if}

  <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Created</dt>
  <dd class="text-phos">{formatDate(project.created_at)}</dd>
  <dt class="font-display text-[13px] uppercase tracking-wide text-phos-dim">Last opened</dt>
  <dd class="text-phos">{formatDate(project.last_opened_at)}</dd>
</dl>
