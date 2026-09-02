// Shared Tailwind utility strings so the form/card/modal components stay
// visually consistent. Amber-CRT terminal theme: bordered not filled,
// cream text with amber-gold as the accent, VT323 on chrome, mono on data.

export const inputClass =
  "rounded-sm border border-line bg-panel-2 px-2.5 py-1.5 font-mono text-[13px] text-phos caret-phos placeholder:text-phos-faint focus:border-phos focus:outline-none";

export const labelClass =
  "flex flex-col gap-1 font-display text-[13px] uppercase tracking-wide text-phos-dim";

// Default action: ghosted until hovered, then picks up the accent.
export const buttonClass =
  "rounded-sm border border-line px-3 py-1.5 font-display text-[14px] uppercase tracking-wide text-phos-dim hover:border-accent hover:text-accent disabled:cursor-default disabled:border-line disabled:text-phos-faint disabled:hover:border-line disabled:hover:text-phos-faint";

// Primary action: present, inverts on hover.
export const primaryButtonClass =
  "rounded-sm border border-accent bg-panel-2 px-3 py-1.5 font-display text-[14px] uppercase tracking-wide text-accent hover:bg-accent hover:text-void disabled:cursor-default disabled:opacity-50 disabled:hover:bg-panel-2 disabled:hover:text-accent";

export const dangerButtonClass =
  "rounded-sm border border-rust px-3 py-1.5 font-display text-[14px] text-rust hover:bg-rust hover:text-void disabled:cursor-default disabled:opacity-50";

export const cardClass = "rounded-sm border border-line bg-panel p-4";
