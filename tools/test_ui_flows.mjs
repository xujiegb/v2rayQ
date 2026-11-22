import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import assert from "node:assert/strict";

const root = new URL("../", import.meta.url);
const binaryDirectory = process.env.CARGO_TARGET_DIR ? join(process.env.CARGO_TARGET_DIR, "debug") : new URL("target/debug/", root).pathname;
const output = new URL("dist/ui-previews/", root).pathname;
const directory = await mkdtemp(join(tmpdir(), "v2rayq-interface-"));
const children = [];

async function launch(binary, database) {
  const processHandle = spawn(join(binaryDirectory, binary), [], {
    env: { ...process.env, [database]: join(directory, binary + ".redb") },
    stdio: ["ignore", "pipe", "inherit"]
  });
  children.push(processHandle);
  return new Promise((resolve, reject) => {
    let buffer = "";
    processHandle.on("error", reject);
    processHandle.once("exit", code => reject(new Error("Panel exited: " + code)));
    processHandle.stdout.on("data", data => {
      buffer += data;
      const match = buffer.match(/http:\/\/127\.0\.0\.1:\d+\/#access=[a-z0-9-]+/);
      if (match) resolve(match[0]);
    });
  });
}
const catalog = async (scope, code) => JSON.parse(await readFile(new URL("locales/" + scope + "/" + code + ".json", root), "utf8"));
let browser;
try {
  await mkdir(output, { recursive: true });
  const client = await launch("v2rayq-web-client", "V2RAYQ_CLIENT_DB");
  const server = await launch("v2rayq-server", "V2RAYQ_SERVER_DB");
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1190, height: 900 }, locale: "en-US", acceptDownloads: true });
  page.setDefaultTimeout(12000);
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  let running = false;
  let switches = 0;
  const runtime = () => ({
    tun: { installed: true, running, can_change: true },
    transparent: { installed: false, running: null, can_change: false },
    traffic: { core_connected: false, upload_bps: null, download_bps: null }
  });
  await page.route("**/api/v1/command", async route => {
    const command = route.request().postDataJSON();
    let result;
    if (command.action === "status") result = { runtime: runtime() };
    if (command.action === "set_network") { running = command.enabled; switches++; result = { runtime: runtime() }; }
    if (command.action === "inspect_network") result = { ip_info: { ip: "192.0.2.44", country: "Example country", city: "Example city", region: "", isp: "Example ISP", checked_at: 1 } };
    if (result) await route.fulfill({ json: { result } });
    else await route.continue();
  });
  await page.goto(client);
  const toggle = page.getByRole("switch");
  await toggle.waitFor();
  await page.waitForFunction(() => !document.querySelector('[role="switch"]').disabled);
  assert.equal(await toggle.getAttribute("aria-checked"), "false");
  await toggle.click();
  await page.waitForFunction(() => document.querySelector('[role="switch"]').getAttribute("aria-checked") === "true");
  await toggle.click();
  await page.waitForFunction(() => document.querySelector('[role="switch"]').getAttribute("aria-checked") === "false");
  assert.equal(switches, 2);
  await page.getByRole("button", { name: "Check IP location", exact: true }).click();
  await page.getByText("192.0.2.44", { exact: true }).waitFor();
  await page.screenshot({ path: join(output, "client-dashboard.png") });
  await page.getByRole("button", { name: "Manage nodes", exact: true }).click();
  await page.getByRole("button", { name: "Manual node", exact: true }).click();
  await page.getByLabel("Protocol", { exact: true }).selectOption("trojan");
  await page.getByLabel("Name", { exact: true }).fill("Example node");
  await page.getByLabel("Server", { exact: true }).fill("example.org");
  await page.getByLabel("Password", { exact: true }).fill("test-secret");
  page.once("dialog", dialog => dialog.dismiss());
  await page.getByRole("button", { name: "Nodes", exact: true }).click();
  assert.equal(await page.getByLabel("Name", { exact: true }).inputValue(), "Example node");
  page.once("dialog", dialog => dialog.dismiss());
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  assert.equal(await page.getByLabel("Name", { exact: true }).inputValue(), "Example node");
  await page.getByRole("button", { name: "Save node", exact: true }).click();
  await page.getByText("Example node", { exact: true }).waitFor();
  await page.getByText("Manage groups", { exact: true }).click();
  await page.getByLabel("Group name", { exact: true }).fill("Travel");
  await page.getByRole("button", { name: "Add group", exact: true }).click();
  await page.getByLabel("Group for Example node", { exact: true }).selectOption({ label: "Travel" });
  await page.waitForFunction(() => document.querySelector('[name="group-filter"]').textContent.includes("Travel · 1"));
  await page.getByLabel("Node groups", { exact: true }).selectOption({ label: "Travel · 1" });
  await page.getByText("Example node", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Manual node", exact: true }).click();
  await page.getByLabel("Protocol", { exact: true }).selectOption("trojan");
  await page.getByLabel("Name", { exact: true }).fill("Grouped addition");
  await page.getByLabel("Server", { exact: true }).fill("grouped.example.org");
  await page.getByLabel("Password", { exact: true }).fill("group-secret");
  await page.getByRole("button", { name: "Save node", exact: true }).click();
  await page.getByText("Grouped addition", { exact: true }).waitFor();
  await page.waitForFunction(() => document.querySelector('[name="group-filter"]').textContent.includes("Travel · 2"));
  await page.screenshot({ path: join(output, "client-groups.png") });
  await page.getByLabel("Node groups", { exact: true }).selectOption("subscriptions");
  await page.getByText("No nodes saved.", { exact: true }).waitFor();
  await page.getByLabel("Node groups", { exact: true }).selectOption("local");
  await page.getByText("Example node", { exact: true }).waitFor();
  await page.getByText("Manage groups", { exact: true }).click();
  await page.getByRole("button", { name: "Rename group", exact: true }).click();
  await page.getByLabel("Group name", { exact: true }).fill("Work");
  await page.getByRole("button", { name: "Save group", exact: true }).click();
  await page.waitForFunction(() => document.querySelector('[name="group-filter"]').textContent.includes("Work · 2"));
  await page.reload();
  await page.getByRole("button", { name: "Manage nodes", exact: true }).click();
  await page.getByLabel("Group for Example node", { exact: true }).selectOption({ label: "Work" });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  let current = "en";
  for (const code of ["zh-CN", "zh-TW", "fr", "en"]) {
    const old = await catalog("client", current), next = await catalog("client", code);
    await page.getByLabel(old.Language, { exact: true }).selectOption(code);
    await page.getByRole("heading", { name: next.Settings, exact: true }).waitFor();
    await page.getByRole("button", { name: next.Overview, exact: true }).click();
    await page.getByRole("heading", { name: next["Live traffic"], exact: true }).waitFor();
    await page.setViewportSize({ width: 375, height: 900 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 375);
    await page.screenshot({ path: join(output, "client-mobile-" + code + ".png"), fullPage: true });
    await page.setViewportSize({ width: 1190, height: 900 });
    await page.getByRole("button", { name: next.Settings, exact: true }).click();
    current = code;
  }

  await page.goto(server);
  await page.getByRole("heading", { name: "Configure with clarity.", exact: true }).waitFor();
  await page.screenshot({ path: join(output, "server-dashboard.png") });
  await page.getByRole("button", { name: "Inbounds", exact: true }).first().click();
  await page.getByRole("button", { name: "Add record", exact: true }).click();
  await page.getByLabel("Name", { exact: true }).fill("Unsaved example");
  current = "en";
  for (const code of ["zh-CN", "zh-TW", "fr", "en"]) {
    const old = await catalog("server", current), next = await catalog("server", code);
    await page.getByLabel(old.Language, { exact: true }).selectOption(code);
    await page.getByRole("heading", { name: next.Inbounds, exact: true }).waitFor();
    assert.equal(await page.getByLabel(next.Name, { exact: true }).inputValue(), "Unsaved example");
    await page.setViewportSize({ width: 375, height: 900 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 375);
    await page.screenshot({ path: join(output, "server-mobile-" + code + ".png"), fullPage: true });
    await page.setViewportSize({ width: 1190, height: 900 });
    current = code;
  }
  page.once("dialog", dialog => dialog.dismiss());
  await page.getByRole("button", { name: "Overview", exact: true }).click();
  assert.equal(await page.getByLabel("Name", { exact: true }).inputValue(), "Unsaved example");
  page.once("dialog", dialog => dialog.accept());
  await page.getByRole("button", { name: "Overview", exact: true }).click();
  await page.getByRole("heading", { name: "Configure with clarity.", exact: true }).waitFor();
  await page.getByRole("button", { name: "Inbounds", exact: true }).first().click();
  await page.getByRole("button", { name: "Add record", exact: true }).click();
  await page.getByLabel("Name", { exact: true }).fill("Saved inbound");
  await page.getByLabel("Protocol", { exact: true }).selectOption("vless");
  await page.getByRole("button", { name: "Save draft", exact: true }).click();
  await page.getByText("Saved inbound", { exact: true }).waitFor();
  await page.getByLabel("Search records", { exact: true }).fill("unmatched");
  assert.equal(await page.getByText("Saved inbound", { exact: true }).isVisible(), false);
  await page.getByLabel("Search records", { exact: true }).fill("Saved");
  await page.getByText("Saved inbound", { exact: true }).waitFor();
  await page.screenshot({ path: join(output, "server-records.png") });
  assert.deepEqual(errors, []);
  console.log("Dashboard switches, groups, four languages, narrow layouts, unsaved forms and search: OK");
} finally {
  await browser?.close();
  for (const child of children) child.kill();
  await rm(directory, { recursive: true, force: true });
}
