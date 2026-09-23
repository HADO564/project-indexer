// Keeps only the newest of overlapping async requests. Each call hands out a
// ticket; once a later ticket has been issued, the earlier one reports stale.
//
//   const isCurrent = ticket();
//   const result = await fetchSomething();
//   if (!isCurrent()) return; // a newer request was made while this one ran
//
// Without it, a slow reply to an old keystroke can land after a quick reply
// to a newer one and overwrite it.
export function latestOnly(): () => () => boolean {
  let newest = 0;
  return () => {
    const mine = ++newest;
    return () => mine === newest;
  };
}
