<script lang="ts">
  import { iconPaths } from "$lib/icons";
  import { iconMd } from "./styles";

  // Renders one glyph from the bundled set, stroked in currentColor so the
  // container decides the colour. An unknown name draws the fallback glyph
  // rather than nothing — a record from a newer build must still render.
  let {
    name,
    class: klass = iconMd,
  }: {
    name: string | null | undefined;
    class?: string;
  } = $props();

  const paths = $derived(iconPaths(name));
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class={klass}
  aria-hidden="true"
>
  {#each paths as d}
    <path {d} />
  {/each}
</svg>
