let clean = false;
const ready = browser.storage.local.get({cleanNavigation: true}).then(settings => {
  clean = settings.cleanNavigation === true;
});
browser.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.cleanNavigation) clean = changes.cleanNavigation.newValue === true;
});
browser.webRequest.onBeforeRequest.addListener(async request => {
  await ready;
  if (!clean || request.method !== "GET" || request.incognito) return {};
  try {
    const cleaned = cleanNavigation(request.url, TRACKING_PARAMETERS);
    return cleaned === request.url ? {} : {redirectUrl: cleaned};
  } catch { return {}; }
}, {urls: ["http://*/*", "https://*/*"], types: ["main_frame"]}, ["blocking"]);
