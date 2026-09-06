<script lang="ts">
  import type { Project } from "$lib/api/types";
  import { customIconName, customIconSrc, isCustomIcon } from "$lib/icons";
  import { swatchVar } from "$lib/palette";
  import BundledIcon from "./BundledIcon.svelte";

  // A project's identity mark: its icon, plus a colour pip.
  //
  // The *pip* carries project colour, not the icon, because a custom icon is
  // an <img> and cannot be tinted — that is what makes colour read identically
  // for bundled and custom icons. A bundled icon additionally tints to the
  // same colour, since it is inline and can.
  let {
    project,
    customIcons,
    size = "sm",
  }: {
    project: Project;
    customIcons: Map<string, string>;
    size?: "sm" | "lg";
  } = $props();

  const color = $derived(swatchVar(project.color));
  const box = $derived(size === "lg" ? "h-7 w-7" : "h-5 w-5");
  const glyph = $derived(size === "lg" ? "h-5 w-5" : "h-4 w-4");
  const pip = $derived(size === "lg" ? "h-2 w-2" : "h-1.5 w-1.5");
  // A custom icon whose file has since been deleted resolves to undefined and
  // falls through to the bundled fallback glyph rather than rendering nothing.
  const custom = $derived(
    project.icon && isCustomIcon(project.icon)
      ? customIcons.get(customIconName(project.icon))
      : undefined,
  );
</script>

<span class={`relative inline-flex shrink-0 items-center justify-center ${box}`}>
  {#if custom}
    <!-- A custom icon renders only through an <img> with a data: URI. It is
         inert regardless of what the sanitizer missed, which is the second of
         two independent security layers. Never {@html}. -->
    <img src={custom} alt="" class={glyph} />
  {:else}
    <span style={`color: ${color}`} class="inline-flex">
      <BundledIcon name={project.icon} class={glyph} />
    </span>
  {/if}
  {#if project.color}
    <span
      class={`absolute -right-0.5 -bottom-0.5 rounded-full ${pip}`}
      style={`background: ${color}`}
      title={project.color}
    ></span>
  {/if}
</span>
