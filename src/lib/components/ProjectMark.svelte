<script lang="ts">
  import type { Project } from "$lib/api/types";
  import { customIconName, customIconSrc, isCustomIcon } from "$lib/icons";
  import { markVar, swatchVar } from "$lib/palette";
  import BundledIcon from "./BundledIcon.svelte";
  import { iconLg, iconMd } from "./styles";

  // A project's identity mark: its icon, plus a colour pip.
  //
  // The *pip* carries project colour, not the icon, because a custom icon is
  // an <img> and cannot be tinted — that is what makes colour read identically
  // for bundled and custom icons. A bundled icon additionally tints to the
  // same colour, since it is inline and can.
  let {
    project,
    customIcons,
    groupColor = null,
    size = "sm",
  }: {
    project: Project;
    customIcons: Map<string, string>;
    // The project's group's palette name, so the mark can inherit it when the
    // project has no colour of its own.
    groupColor?: string | null;
    size?: "sm" | "lg";
  } = $props();

  // Icon tint cascades project -> group -> neutral, so a project in a coloured
  // group reads as belonging to it rather than sitting grey beside a coloured
  // sidebar entry.
  const color = $derived(markVar(project.color, groupColor));
  // The box is exactly the glyph, so the mark carries no padding of its own —
  // spacing is the row's `gap` and nothing else. Previously the box was larger
  // than the glyph by 2px at sm and 4px at lg, which read as uneven and did
  // not scale between the two sizes.
  const glyph = $derived(size === "lg" ? iconLg : iconMd);
  const pip = $derived(size === "lg" ? "h-3 w-3" : "h-2 w-2");
  // A custom icon whose file has since been deleted resolves to undefined and
  // falls through to the bundled fallback glyph rather than rendering nothing.
  const custom = $derived(
    project.icon && isCustomIcon(project.icon)
      ? customIcons.get(customIconName(project.icon))
      : undefined,
  );
</script>

<span class={`relative inline-flex shrink-0 items-center justify-center ${glyph}`}>
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
    <!-- The pip carries the project's *own* colour only. It never inherits the
         group's, or "has its own colour" and "inherits its group's" would look
         identical. It is also the only way colour reads on a custom icon,
         which is an <img> and cannot be tinted. -->
    <span
      class={`absolute -right-0.5 -bottom-0.5 rounded-full ${pip}`}
      style={`background: ${swatchVar(project.color)}`}
      title={project.color}
    ></span>
  {/if}
</span>
