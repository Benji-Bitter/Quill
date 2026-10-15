const form = document.querySelector("#search");
const input = document.querySelector("#query");
const results = document.querySelector("#results");
const status = document.querySelector("#status");
let generation = 0;
const extensionAvailable = typeof browser !== "undefined" && !!browser.history;
if (!extensionAvailable) {
  document.querySelector("footer span").textContent = "Development host · extension unavailable";
  document.querySelector("footer a").hidden = true;
  input.placeholder = "Enter a web address";
  status.textContent = "Extension unavailable. Local search and Quill URL cleaning are not active.";
}
form.addEventListener("submit", async event => {
  event.preventDefault();
  const current = ++generation;
  results.replaceChildren(); status.textContent = "";
  const query = input.value.trim(); if (!query) return;
  const url = address(query);
  if (url) { location.assign(url); return; }
  if (!extensionAvailable) { status.textContent = "Local search requires the Quill extension. Enter a full HTTP or HTTPS address."; return; }
  const settings = await browser.storage.local.get({localSearch: true});
  if (!settings.localSearch || browser.extension.inIncognitoContext) { status.textContent = "Local history search is disabled. Enter a full HTTP or HTTPS address."; return; }
  try {
    const [history, bookmarks] = await Promise.all([
      browser.history.search({text: query, startTime: 0, maxResults: 30}),
      browser.bookmarks.search(query)
    ]);
    if (current !== generation) return;
    const seen = new Set();
    for (const result of [...bookmarks, ...history]) {
      if (!result.url || !/^https?:\/\//.test(result.url) || seen.has(result.url)) continue;
      seen.add(result.url);
      const item = document.createElement("li"); const link = document.createElement("a"); const detail = document.createElement("small");
      link.href = result.url; link.textContent = result.title || result.url; detail.textContent = result.url;
      item.append(link, detail); results.append(item);
      if (seen.size === 20) break;
    }
    status.textContent = seen.size ? `${seen.size} local results` : "No local results. Public web search is not connected yet.";
  } catch { status.textContent = "Local search is unavailable in this session."; }
});
