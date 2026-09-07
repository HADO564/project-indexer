<script lang="ts">
  import { SWATCHES, swatchVar } from "$lib/palette";
  import { labelClass } from "./styles";

  // Binds a palette *name*, never a literal. swatchVar can only ever return
  // one of nine fixed var(--color-*) strings, so nothing arbitrary reaches the
  // style attribute.
  let {
    value = $bindable<string | null>(null),
    label = "Colour",
    // A project may have no colour; a group must have one.
    allowNone = false,
  }: {
    value?: string | null;
    label?: string;
    allowNone?: boolean;
  } = $props();
</script>

<div class={labelClass}>
  {label}
  <div class="flex flex-wrap items-center gap-1.5">
    {#if allowNone}
      <button
        type="button"
        onclick={() => (value = null)}
        title="No colour"
        aria-label="No colour"
        aria-pressed={value === null}
        class={`h-8 w-8 rounded-full border text-[13px] text-phos-faint ${
          value === null ? "border-phos" : "border-line"
        }`}
      >
        —
      </button>
    {/if}
    {#each SWATCHES as swatch}
      <button
        type="button"
        onclick={() => (value = swatch)}
        title={swatch}
        aria-label={swatch}
        aria-pressed={value === swatch}
        class={`h-8 w-8 rounded-full border-2 ${
          value === swatch ? "border-phos" : "border-transparent"
        }`}
        style={`background: ${swatchVar(swatch)}`}
      ></button>
    {/each}
  </div>
</div>
