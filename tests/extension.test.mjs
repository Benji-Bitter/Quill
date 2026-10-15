import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {test} from "node:test";
import vm from "node:vm";

const context = vm.createContext({URL, URLSearchParams});
for (const file of ["clean-url.js", "navigation.js"]) {
  vm.runInContext(readFileSync(new URL(`../browser/extension/${file}`, import.meta.url), "utf8"), context);
}
const parameters = JSON.parse(readFileSync(new URL("../privacy/tracking-parameters.json", import.meta.url), "utf8"));

test("navigation cleaning preserves unknown query encoding", () => {
  assert.equal(context.cleanNavigation("https://example.org/?a=%2f+%20&utm_source=x&b=2&a=3#part", parameters), "https://example.org/?a=%2f+%20&b=2&a=3#part");
  assert.equal(context.cleanNavigation("https://example.org/?%75tm_source=x", parameters), "https://example.org/");
});
test("every configured parameter is removed case-insensitively", () => {
  for (const name of parameters) {
    assert.equal(context.cleanNavigation(`https://example.org/?${name.toUpperCase()}=x`, parameters), "https://example.org/");
  }
});
test("addresses include host ports and never executable schemes", () => {
  assert.equal(context.address("localhost:8000/page"), "https://localhost:8000/page");
  assert.equal(context.address("example.org:8443"), "https://example.org:8443/");
  assert.equal(context.address("https://example.org/"), "https://example.org/");
  for (const input of ["javascript:alert(1)", "file:///etc/passwd", "https://u:p@example.org", "what is rust"]) assert.equal(context.address(input), null);
});
test("cleaning is idempotent on unusual inputs", () => {
  for (const value of ["%", "%FF", "a&gclid=x", "💡", "a=b", "", "%26utm_source=x"]) {
    const once = context.cleanNavigation(`https://example.org/?q=${value}&fbclid=a`, parameters);
    assert.equal(context.cleanNavigation(once, parameters), once);
  }
});
