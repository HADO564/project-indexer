<script lang="ts">
  import { SWATCHES, isHexColor, swatchVar } from "$lib/palette";
  import { labelClass } from "./styles";

  // Binds a palette *name*, never a literal. swatchVar can only ever return
  // one of nine fixed var(--color-*) strings, so nothing arbitrary reaches the
  // style attribute.
  let {
    value = $bindable<string | null>(null),
    label = "Colour",
    // A project may have no colour; a group must have one.
    allowNone = false,
    // A project may also store an exact hex literal from the colour picker;
    // a group stays palette-only, matching is_valid_project_color in core —
    // its colour drives sidebar entries and card edges, where staying inside
    // the theme matters most.
    allowCustom = false,
  }: {
    value?: string | null;
    label?: string;
    allowNone?: boolean;
    allowCustom?: boolean;
  } = $props();

  // What the colour input shows. It always needs a concrete hex, so a palette
  // name or an empty value falls back to a neutral mid-grey rather than
  // leaving the swatch black.
  const custom = $derived(isHexColor(value) ? (value as string) : "#8899aa");
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

    {#if allowCustom}
      <label
        class="relative inline-flex h-8 w-8 cursor-pointer items-center justify-center rounded-full border-2"
        style={`border-color: ${isHexColor(value) ? "var(--color-phos)" : "transparent"}; background: ${custom}`}
        title="Custom colour"
      >
        <input
          type="color"
          value={custom}
          oninput={(e) => (value = e.currentTarget.value)}
          class="absolute inset-0 cursor-pointer opacity-0"
          aria-label="Custom colour"
        />
        <span
          class="pointer-events-none font-display text-[13px] leading-none text-void mix-blend-difference"
          aria-hidden="true">+</span
        >
      </label>
    {/if}
  </div>

  {#if allowCustom && isHexColor(value)}
    <p class="font-mono text-[12px] normal-case text-phos-faint">{value}</p>
  {/if}
</div>
