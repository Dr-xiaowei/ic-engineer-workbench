import { test } from "node:test";
import assert from "node:assert/strict";
import { classifyWindows } from "./windows-environment.mjs";

test("Server NT 10 is accepted only as generic Windows evidence", () => {
  const server = { BuildNumber: "20348", ProductType: 3 };
  assert.throws(() => classifyWindows(server), /Win10/);
  assert.deepEqual(classifyWindows(server, "windows"), { family: "windows-server", build: 20348, scope: "windows", win10Evidence: false });
});
test("Windows 11 cannot satisfy Win10 acceptance", () => {
  const client = { BuildNumber: "26100", ProductType: 1 };
  assert.throws(() => classifyWindows(client), /Win10/);
  assert.equal(classifyWindows(client, "windows").family, "windows-11");
});
test("Supported Win10 clients and malformed environment records", () => {
  for (const build of [19044, 19045]) assert.equal(classifyWindows({ BuildNumber: build, ProductType: 1 }).win10Evidence, true);
  for (const system of [{}, { BuildNumber: 19043, ProductType: 1 }, { BuildNumber: "bad", ProductType: 3 }, { BuildNumber: 20348 }]) assert.throws(() => classifyWindows(system, "windows"));
});
