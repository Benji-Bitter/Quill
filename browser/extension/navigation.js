function address(value) {
  const explicit = /^https?:\/\//i.test(value);
  if (!explicit && (!/^[^\s/]+\.[^\s/]+/.test(value) && !/^localhost(?::\d+)?(?:\/|$)/.test(value))) return null;
  try {
    const url = new URL(explicit ? value : `https://${value}`);
    return ["https:", "http:"].includes(url.protocol) && !url.username && !url.password ? url.href : null;
  } catch { return null; }
}
