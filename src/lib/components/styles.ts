// Shared Tailwind utility strings so the form/card/modal components stay visually consistent
// without repeating long class lists in every file.

export const inputClass =
  "rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";

export const labelClass = "flex flex-col gap-1 text-sm text-gray-700 dark:text-gray-300";

export const buttonClass =
  "rounded-md bg-gray-200 px-4 py-2 text-sm font-medium text-gray-900 hover:bg-gray-300 disabled:cursor-default disabled:opacity-60 dark:bg-gray-700 dark:text-gray-100 dark:hover:bg-gray-600";

export const primaryButtonClass =
  "rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-default disabled:opacity-60";

export const dangerButtonClass =
  "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-default disabled:opacity-60";

export const cardClass = "rounded-lg bg-white p-4 shadow-sm dark:bg-gray-800";
