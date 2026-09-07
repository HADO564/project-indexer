<script lang="ts">
  import { swatchVar } from "$lib/palette";
  import BundledIcon from "./BundledIcon.svelte";
  import { iconMd } from "./styles";

  // One sidebar entry: icon, colour, label, count. The count is not optional
  // by design — the invariant is that no view can hide projects without
  // saying so, which means an empty group shows a 0 rather than disappearing.
  let {
    icon,
    label,
    count,
    color = null,
    selected = false,
    onSelect,
  }: {
    icon: string;
    label: string;
    count: number;
    color?: string | null;
    selected?: boolean;
    onSelect: () => void;
  } = $props();
</script>

<button
  type="button"
  onclick={onSelect}
  aria-current={selected ? "page" : undefined}
  class={`flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left font-display text-[15px] ${
    selected ? "bg-panel-2 text-phos" : "text-phos-dim hover:bg-panel-2 hover:text-phos"
  }`}
>
  <span class="shrink-0" style={color ? `color: ${swatchVar(color)}` : undefined}>
    <BundledIcon name={icon} class={iconMd} />
  </span>
  <span class="min-w-0 flex-1 truncate">{label}</span>
  <span class="shrink-0 text-[13px] tabular-nums text-phos-faint">{count}</span>
</button>
