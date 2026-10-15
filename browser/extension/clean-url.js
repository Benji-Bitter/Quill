/* Used only for top-level GET navigation, never API requests or form posts. */
function cleanNavigation(input, parameters) {
  const url = new URL(input);
  if (!["http:", "https:"].includes(url.protocol)) return input;
  const query = url.search.slice(1).split("&").filter(part => {
    const name = new URLSearchParams(part).keys().next().value;
    return !name || !parameters.includes(name.toLowerCase());
  }).join("&");
  url.search = query ? `?${query}` : "";
  return url.href;
}
