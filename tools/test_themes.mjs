// Real frontend assets + Rust RPC. The LuCI/Cockpit chrome below is a labelled preview harness.
import { createServer } from "node:http";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, mkdir, rm } from "node:fs/promises";
import { join, resolve, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
const { chromium } = await import(process.env.V2RAYQ_PLAYWRIGHT || "playwright");
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const bin = join(process.env.CARGO_TARGET_DIR || join(root, "target"), "debug");
const output = process.env.V2RAYQ_PREVIEW_OUT || join(root, "dist/ui-previews");
const directory = await mkdtemp(join(tmpdir(), "v2rayq-theme-"));
const env = { ...process.env, V2RAYQ_CLIENT_DB: join(directory, "client.redb"), V2RAYQ_SERVER_DB: join(directory, "server.redb") };
const children = [];
function rpc(binary, command) {
  assert(["v2rayq-cli", "v2rayq-server"].includes(binary));
  const value = JSON.parse(execFileSync(join(bin, binary), ["--rpc"], { env, input: JSON.stringify(command), encoding: "utf8" }));
  if (value.error) throw Error(value.error);
  return value.result;
}
const node = rpc("v2rayq-cli", { action: "add_link", link: "trojan://preview-secret@edge.example.net:443#Tokyo%20%2F%20Premium" }).profile.selected_node;
rpc("v2rayq-cli", { action: "save_node_group", id: null, name: "日常使用" });
const group = rpc("v2rayq-cli", { action: "profile" }).profile.groups[0].id;
rpc("v2rayq-cli", { action: "assign_node_group", node_id: node, group_id: group });
for (const [section, value] of [["inbounds", [{ tag: "VLESS · Japan", type: "vless", listen: "::", listen_port: 443 }]], ["users", [{ name: "Personal", enabled: true }]], ["nodes", [{ name: "Tokyo edge", protocol: "vless", server: "edge.example.net", server_port: 443, security: "reality" }]]]) rpc("v2rayq-server", { action: "save", section, revision: 0, value });
for (const binary of ["v2rayq-cli", "v2rayq-server"]) rpc(binary, { action: "save_language", language: "zh-CN" });
async function launch(binary) {
  const child = spawn(join(bin, binary), [], { env, stdio: ["ignore", "pipe", "inherit"] }); children.push(child);
  return new Promise((resolve, reject) => {
    child.on("error", reject); child.once("exit", code => reject(Error("Panel exited " + code)));
    child.stdout.on("data", data => { const url = data.toString().match(/http:\/\/127\.0\.0\.1:\d+\/#access=[a-f0-9-]+/); if (url) resolve(url[0]); });
  });
}
const pluginRoot = join(directory, "cockpit");
execFileSync("sh", ["frontends/cockpit/install.sh", pluginRoot], { cwd: root });
const clientAssets = join(root, "frontends/luci/htdocs/luci-static/resources/v2rayq");
execFileSync("python3", ["tools/prepare_locales.py", "client", join(directory, "locales")], { cwd: root });
const bridge = `window.cockpit={spawn(args){let input="";return{input(value){input=value;},then(ok,fail){return fetch('/bridge/'+args[0].split('/').at(-1),{method:'POST',body:input}).then(r=>r.text()).then(ok,fail);}};}};`;
const chrome = (name, content) => `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><style>body{margin:0;font:14px system-ui;color:#243447;background:#edf1f5}header{height:54px;display:flex;align-items:center;justify-content:space-between;padding:0 26px;background:#172635;color:#f1f6fc}header strong{font-size:18px}header small{color:#a7b8cb}.shell{display:grid;grid-template-columns:174px 1fr;min-height:calc(100vh - 54px)}aside{padding:25px 18px;background:#e5ebf2}aside p{padding:10px;border-radius:6px}aside .active{background:#d2deed;font-weight:650}main{min-width:0;padding:18px}iframe{border:0;width:100%;height:1050px;border-radius:10px}html[data-host-dark] body{background:#101721;color:#d9e5f4}html[data-host-dark] aside{background:#172331}html[data-host-dark] aside .active{background:#2b3d54}</style><header><strong>${name}</strong><small>宿主框架预览 · 示例配置 · 实际 v2rayQ 插件界面</small></header><div class="shell"><aside><p>系统概览</p><p>网络</p><p>服务</p><p class="active">v2rayQ</p></aside><main>${content}</main></div></html>`;
const server = createServer(async (req, res) => {
  try {
    const path = new URL(req.url, "http://localhost").pathname;
    if (path.startsWith("/bridge/")) {
      let data = ""; for await (const chunk of req) data += chunk;
      if (data.length > 8388608) throw Error("Preview request too large");
      res.setHeader("Content-Type", "application/json"); res.end(JSON.stringify({ result: rpc(path.slice(8), JSON.parse(data)) })); return;
    }
    if (path.endsWith("/base1/cockpit.js")) { res.setHeader("Content-Type", "text/javascript"); res.end(bridge); return; }
    if (path === "/cockpit-client" || path === "/cockpit-server") {
      const side = path.slice(9); res.setHeader("Content-Type", "text/html");
      res.end(chrome("Cockpit · " + (side === "client" ? "客户端" : "服务端"), `<iframe title="v2rayQ" src="/cockpit/v2rayq-${side}/${side}.html"></iframe>`)); return;
    }
    if (path === "/luci") {
      res.setHeader("Content-Type", "text/html");
      res.end(chrome("LuCI · OpenWrt", '<div id="luci-view"></div><script type="module" src="/luci-entry.mjs"></script>')); return;
    }
    if (path === "/luci-entry.mjs") {
      const source = await readFile(join(root, "frontends/luci/htdocs/luci-static/resources/view/v2rayq/client.js"), "utf8");
      const boot = `const view={extend:value=>value};const rpc={declare:definition=>async(...args)=>{if(definition.method==='access')return false;return fetch('/bridge/v2rayq-cli',{method:'POST',body:args[0]}).then(r=>r.json());}};const L={resource:path=>'/luci-static/resources/'+path};const E=(tag,attrs={},children=[])=>{const n=document.createElement(tag);for(const [k,v] of Object.entries(attrs))n.setAttribute(k,v);n.append(...children);return n;};const plugin=new Function('view','rpc','L','E',${JSON.stringify(source)})(view,rpc,L,E);document.getElementById('luci-view').append(plugin.render(await plugin.load()));`;
      res.setHeader("Content-Type", "text/javascript"); res.end(boot); return;
    }
    let file;
    if (path.startsWith("/cockpit/")) file = join(pluginRoot, path.slice(9));
    else if (path.startsWith("/luci-static/resources/v2rayq/locales/")) file = join(directory, "locales", path.split("/").at(-1));
    else if (path.startsWith("/luci-static/resources/v2rayq/")) file = join(clientAssets, path.split("/").at(-1));
    else { res.writeHead(404); res.end(); return; }
    assert(file.startsWith(directory) || file.startsWith(clientAssets));
    res.setHeader("Content-Type", /\.m?js$/.test(file) ? "text/javascript" : file.endsWith(".json") ? "application/json" : file.endsWith(".css") ? "text/css" : "text/html");
    res.end(await readFile(file));
  } catch (error) { res.writeHead(500); res.end(String(error)); }
});
let browser;
try {
  await mkdir(output, { recursive: true });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const host = "http://127.0.0.1:" + server.address().port;
  const clientUrl = await launch("v2rayq-web-client"), serverUrl = await launch("v2rayq-server");
  const options = { headless: true };
  if (process.env.V2RAYQ_CHROMIUM_MODULE) {
    const { default: engine } = await import(process.env.V2RAYQ_CHROMIUM_MODULE);
    options.executablePath = await engine.executablePath(); options.args = engine.args;
  }
  browser = await chromium.launch(options);
  const page = await browser.newPage({ viewport: { width: 1360, height: 1130 }, locale: "zh-CN" });
  const errors = []; page.on("pageerror", error => errors.push(error.message));
  for (const [name, url, side, framed] of [["webui-client", clientUrl, "client", false], ["webui-server", serverUrl, "server", false], ["luci-client", host + "/luci", "client", false], ["cockpit-client", host + "/cockpit-client", "client", true], ["cockpit-server", host + "/cockpit-server", "server", true]]) {
    await page.goto(url);
    const app = framed ? page.frameLocator("iframe") : page;
    if (side === "client") await app.getByRole("button", { name: "设置", exact: true }).click();
    await app.getByLabel("主题", { exact: true }).waitFor();
    if (side === "server") {
      await app.getByRole("button", { name: "入站管理", exact: true }).first().click();
      await app.getByRole("button", { name: "添加配置", exact: true }).click();
      await app.getByLabel("名称", { exact: true }).fill("Theme draft");
      await app.getByLabel("主题", { exact: true }).selectOption("dark");
      await app.locator('html[data-theme="dark"]').waitFor();
      assert.equal(await app.getByLabel("名称", { exact: true }).inputValue(), "Theme draft");
      page.once("dialog", dialog => dialog.accept());
      await app.getByRole("button", { name: "信息", exact: true }).click();
    }
    for (const theme of ["light", "dark"]) {
      await app.getByLabel("主题", { exact: true }).selectOption(theme);
      const selector = side === "client" ? '.vq-client[data-theme="' + theme + '"]' : 'html[data-theme="' + theme + '"]';
      await app.locator(selector).waitFor();
      await page.evaluate(dark => { document.documentElement.toggleAttribute("data-host-dark", dark); }, theme === "dark");
      await page.reload();
      const scope = framed ? page.frameLocator("iframe") : page;
      await scope.locator(selector).waitFor();
      await page.evaluate(dark => { document.documentElement.toggleAttribute("data-host-dark", dark); }, theme === "dark");
      if (side === "client") await scope.getByRole("heading", { name: "实时流量", exact: true }).waitFor();
      await page.screenshot({ path: join(output, name + "-" + theme + ".png"), fullPage: true });
      if (side === "client") await scope.getByRole("button", { name: "设置", exact: true }).click();
    }
    await app.getByLabel("主题", { exact: true }).selectOption("system");
    await page.emulateMedia({ colorScheme: "light" });
    const surface = app.locator(side === "client" ? ".vq-client" : "body");
    const light = await surface.evaluate(element => getComputedStyle(element).backgroundColor);
    await page.emulateMedia({ colorScheme: "dark" });
    const dark = await surface.evaluate(element => getComputedStyle(element).backgroundColor);
    assert.notEqual(light, dark, name + " follows system changes without reload");
    if (name === "luci-client") assert.equal(await page.locator("html").getAttribute("data-theme"), null, "LuCI host theme stays independent");
    await page.setViewportSize({ width: 390, height: 900 });
    if (!framed) assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 390);
    await page.setViewportSize({ width: 1360, height: 1130 });
    await page.emulateMedia({ colorScheme: "light" });
  }
  assert.deepEqual(errors, []);
  console.log("Themes: five Web/plugin surfaces, persistence, system changes, host isolation and screenshots: OK");
} finally {
  await browser?.close(); server.close(); for (const child of children) child.kill(); await rm(directory, { recursive: true, force: true });
}
