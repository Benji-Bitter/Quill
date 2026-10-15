(async () => {
  const defaults = {cleanNavigation: true, localSearch: true};
  const settings = await browser.storage.local.get(defaults);
  for (const name of Object.keys(defaults)) {
    const checkbox = document.getElementById(name); checkbox.checked = settings[name];
    checkbox.addEventListener("change", async () => {
      try { await browser.storage.local.set({[name]: checkbox.checked}); document.getElementById("status").textContent = "Saved on this device."; }
      catch { document.getElementById("status").textContent = "Could not save this setting."; }
    });
  }
})();
