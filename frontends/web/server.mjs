import { applyTheme, themeChoices } from "./appearance.js";
import { loadLocalization } from "./i18n.js";
import { transport } from "./transport.mjs";
import { fields, defaults, choices } from "./server-fields.mjs";

const serverPages = [
  ["overview", "Overview", "⌂", "Status"],
  ["inbounds", "Inbounds", "⇥", "Endpoints"],
  ["users", "Users", "♙", "Endpoints"],
  ["outbounds", "Outbounds", "⇤", "Endpoints"],
  ["nodes", "Nodes", "◇", "Endpoints"],
  ["services", "Services", "▤", "System"],
  ["tls", "TLS", "♢", "Security"],
  ["reality", "REALITY", "◇", "Security"],
  ["system", "System", "▧", "Host"],
  ["routes", "Routes", "⇄", "Policies"],
  ["dns", "DNS", "◎", "Policies"],
  ["administrators", "Administrators", "♙", "Administration"],
  ["settings", "Settings", "⚙", "Administration"]
];
applyTheme(document.documentElement, "system");
const i18n = await loadLocalization(new URL("./locales/", import.meta.url), "server");
const nav = document.getElementById("navigation");
const heading = document.getElementById("heading");
const notice = document.getElementById("notice");
const view = document.getElementById("view");
let send;
let active = 0;
let dirty = false;
let savingPage = false;
window.addEventListener("beforeunload", event => { if (dirty) { event.preventDefault(); event.returnValue = ""; } });

function element(tag, className = "", text) {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function localizedElement(tag, className, key, values) {
  return i18n.bind(element(tag, className), key, values);
}

function updateLanguage() {
  i18n.update(document);
  document.getElementById("navigation").setAttribute("aria-label", i18n.text("Server navigation"));
}

function button(label, action, className = "") {
  const node = label ? localizedElement("button", className, label) : element("button", className);
  node.type = "button";
  node.onclick = action;
  return node;
}

function panel(title, ...children) {
  const node = element("section", "panel");
  node.append(localizedElement("h2", "", title), ...children);
  return node;
}

function infoRow(name, value) {
  const row = element("div", "info-row");
  row.append(localizedElement("span", "", name), localizedElement("strong", "", String(value ?? "Unavailable")));
  return row;
}

function overview(status) {
  const hero = element("section", "server-hero");
  const copy = element("div");
  copy.append(localizedElement("span", "eyebrow", "Server workspace"),
    localizedElement("h2", "", "Configure with clarity."),
    localizedElement("p", "", "Review your endpoints, security profiles and users in one place."));
  const badge = localizedElement("span", "core-badge", status.core_connected ? "Connected" : "Awaiting adapter");
  hero.append(copy, badge);
  const topline = element("div", "metrics");
  for (const [key, label, glyph] of [["inbounds", "Inbounds", "⇥"], ["users", "Users", "♙"], ["outbounds", "Outbounds", "⇤"], ["nodes", "Nodes", "◇"]]) {
    const count = panel(label, element("strong", "metric", String(status.counts[key])));
    count.prepend(element("span", "metric-icon", glyph));
    count.append(button("Open", () => open(key)));
    topline.append(count);
  }
  const setup = panel("Configuration workflow");
  const steps = [
    ["01", "Security", "Add TLS certificates or a REALITY profile.", [["tls", "TLS"], ["reality", "REALITY"]]],
    ["02", "Endpoints", "Configure inbounds, outbounds and nodes.", [["inbounds", "Inbounds"], ["nodes", "Nodes"]]],
    ["03", "Users", "Review users and save your service drafts.", [["users", "Users"], ["services", "Services"]]]
  ];
  for (const [number, title, description, links] of steps) {
    const row = element("div", "workflow-step");
    const copy = element("div");
    copy.append(localizedElement("strong", "", title), localizedElement("p", "", description));
    const actions = element("div", "actions");
    for (const [key, label] of links) actions.append(button(label, () => open(key)));
    row.append(element("span", "step-number", number), copy, actions);
    setup.append(row);
  }
  return [hero, topline, setup, panel("Deployment",
    infoRow("Core connection", status.core_connected ? "Connected" : "Awaiting adapter"),
    infoRow("Configuration", "Saved drafts"),
    infoRow("Administrative access", "Local launch session or Cockpit"))];
}

function editor(section, title, draft, profiles = {}) {
  const isList = Array.isArray(draft.value);
  const layout = element("div", "editor-layout");
  const controls = element("div", "actions");
  const form = element("form", "editor-form");
  const inputs = element("fieldset", "editor-fields");
  const advanced = element("details", "raw-editor");
  const textarea = element("textarea");
  textarea.setAttribute("aria-label", i18n.text(title));
  textarea.dataset.i18nLabel = title;
  textarea.spellcheck = false;
  textarea.oninput = () => { dirty = true; };
  advanced.append(localizedElement("summary", "", "Edit JSON"), textarea);
  const label = localizedElement("h2", "", "Edit configuration");
  const edit = element("section", "panel edit");
  const list = panel(title);
  let current = {};
  let index = null;
  let saving = false;

  function refreshFields() {
    const value = key => {
      const input = inputs.elements.namedItem(key);
      return input?.value || "";
    };
    for (const [key, , , visible] of fields[section]) {
      if (!visible) continue;
      const input = inputs.elements.namedItem(key);
      if (!input) continue;
      const [parent, allowed] = visible;
      input.disabled = !allowed.includes(value(parent));
      if (parent === "transport" && !["vless", "vmess", "trojan"].includes(value("protocol") || value("type"))) input.disabled = true;
      input.parentElement.hidden = input.disabled;
    }
    const options = inputs.querySelector("datalist");
    if (options) {
      const names = profiles[value("security")];
      options.replaceChildren(...(Array.isArray(names) ? names : []).map(name => {
        const option = element("option");
        option.value = name;
        return option;
      }));
    }
  }

  function drawFields() {
    inputs.replaceChildren();
    for (const [key, title, kind] of fields[section]) {
      const value = key === "protocol" ? current.protocol ?? current.type : current[key];
      const compatible = value === undefined || (kind === "list" && Array.isArray(value) && value.every(item => typeof item === "string"))
        || (kind === "port" && typeof value === "number") || (kind === "boolean" && typeof value === "boolean")
        || (kind !== "list" && kind !== "port" && kind !== "boolean" && typeof value === "string");
      if (!compatible) continue; // Keep structured or legacy values editable in the JSON view.
      const wrapper = element("label", "editor-field");
      wrapper.append(localizedElement("span", "", title));
      const select = choices[kind];
      const input = element(select ? "select" : "input");
      input.name = key;
      if (select) {
        input.setAttribute("aria-label", i18n.text(title)); input.dataset.i18nLabel = title;
        const values = new Map(Object.entries(select));
        if (value && !values.has(value)) values.set(value, value);
        const empty = localizedElement("option", "", "Select…");
        empty.value = "";
        input.append(empty);
        for (const [id, label] of values) {
          const option = localizedElement("option", "", label);
          option.value = id;
          input.append(option);
        }
        input.value = value ?? "";
      } else if (kind === "boolean") {
        input.type = "checkbox";
        input.checked = value === true;
        input.indeterminate = value === undefined;
      } else {
        input.type = kind === "port" ? "number" : kind === "secret" ? "password" : "text";
        if (kind === "port") { input.min = "1"; input.max = "65535"; }
        if (kind === "secret") input.autocomplete = "new-password";
        if (kind === "list") { input.placeholder = i18n.text("Separate values with commas"); input.dataset.i18nPlaceholder = "Separate values with commas"; }
        if (kind === "profile") input.setAttribute("list", "security-profiles");
        input.value = Array.isArray(value) ? value.join(", ") : value ?? "";
      }
      input.oninput = () => { input.dataset.edited = "true"; dirty = true; if (select) refreshFields(); };
      input.onchange = input.oninput;
      wrapper.append(input);
      inputs.append(wrapper);
    }
    if (["nodes", "inbounds", "outbounds"].includes(section)) {
      const options = element("datalist");
      options.id = "security-profiles";
      inputs.append(options);
    }
    refreshFields();
  }

  function fieldValues() {
    const value = { ...current };
    for (const [key, , kind, visible] of fields[section]) {
      const input = inputs.elements.namedItem(key);
      if (!input || input.indeterminate) continue;
      if (input.disabled) {
        const parent = inputs.elements.namedItem(visible?.[0] || "");
        const protocol = inputs.elements.namedItem("protocol");
        if (parent?.dataset.edited === "true" || (visible?.[0] === "transport" && protocol?.dataset.edited === "true")) delete value[key];
        continue;
      }
      if (input.dataset.edited !== "true") continue;
      if (kind === "boolean") value[key] = input.checked;
      else if (!input.value.trim()) delete value[key];
      else if (kind === "list") value[key] = input.value.split(",").map(item => item.trim()).filter(Boolean);
      else value[key] = kind === "port" ? input.valueAsNumber : kind === "secret" ? input.value : input.value.trim();
    }
    return value;
  }

  function jsonValue() {
    const value = JSON.parse(textarea.value);
    if (value === null || Array.isArray(value) || typeof value !== "object") {
      throw new Error("Enter one JSON object.");
    }
    return value;
  }

  advanced.querySelector("summary").onclick = event => {
    event.preventDefault();
    if (advanced.open) {
      try {
        current = jsonValue();
        drawFields();
      } catch {
        notice.textContent = i18n.text("Enter a valid JSON object before returning to the form.");
        return;
      }
    } else {
      if (!form.reportValidity()) return;
      textarea.value = JSON.stringify(fieldValues(), null, 2);
    }
    advanced.open = !advanced.open;
    inputs.hidden = advanced.open;
    notice.textContent = "";
  };

  function start(value, position) {
    if (saving) return;
    if (dirty && !window.confirm(i18n.text("Discard unsaved changes?"))) return;
    dirty = false;
    index = position;
    current = value;
    advanced.open = false;
    inputs.hidden = false;
    drawFields();
    i18n.bind(label, position === null && isList ? "New record" : "Edit configuration");
    edit.hidden = false;
    if (isList) inputs.querySelector("input")?.focus();
  }

  async function save(value) {
    if (saving) return;
    saving = true;
    savingPage = true;
    inputs.disabled = true;
    textarea.disabled = true;
    controls.querySelectorAll("button").forEach(control => { control.disabled = true; });
    try {
      const result = await send({ action: "save", section, revision: draft.revision, value });
      draft = { revision: result.revision, value };
      dirty = false;
      if (!layout.isConnected) return;
      notice.textContent = i18n.text("Draft saved.");
      if (isList) { edit.hidden = true; drawList(); }
      else current = value;
    } catch (error) {
      if (layout.isConnected) notice.textContent = i18n.message(error.message);
    } finally {
      saving = false;
      savingPage = false;
      inputs.disabled = false;
      textarea.disabled = false;
      controls.querySelectorAll("button").forEach(control => { control.disabled = false; });
    }
  }

  let query = "";
  function drawList() {
    const header = element("div", "list-header");
    header.append(localizedElement("h2", "", "{count} saved records", { count: draft.value.length }),
      button("Add record", () => start(defaults[section] || {}, null), "primary"));
    const search = element("input", "record-search");
    search.type = "search";
    search.placeholder = i18n.text("Search records");
    search.setAttribute("aria-label", i18n.text("Search records"));
    search.dataset.i18nPlaceholder = "Search records";
    search.dataset.i18nLabel = "Search records";
    search.value = query;
    const rows = element("div", "record-rows");
    search.oninput = () => {
      query = search.value;
      for (const row of rows.children) row.hidden = !row.dataset.search.includes(query.trim().toLowerCase());
    };
    list.replaceChildren(header, search, rows);
    if (!draft.value.length) list.append(localizedElement("p", "empty-state", "No records yet. Add one to get started."));
    draft.value.forEach((record, i) => {
      const row = element("div", "info-row");
      const candidate = record.name || record.tag || record.username || record.outbound;
      const named = typeof candidate === "string" && candidate;
      const name = named ? candidate : i18n.format("Record {index}", { index: i + 1 });
      const operations = element("div", "actions");
      operations.append(button("Edit", () => start(record, i)), button("Delete", () => {
        if (!saving && window.confirm(i18n.format("Delete {name}?", { name }))) save(draft.value.filter((_, position) => position !== i));
      }, "danger"));
      const details = element("div", "record-title");
      details.append(named ? element("strong", "", name)
        : localizedElement("strong", "", "Record {index}", { index: i + 1 }));
      if (section === "nodes") {
        const endpoint = typeof record.server === "string" && record.server_port ? `${record.server}:${record.server_port}` : record.server;
        const protocol = record.protocol || record.type;
        const summary = [choices.protocol[protocol] || protocol, endpoint, i18n.text(choices.security[record.security] || "")]
          .filter(value => typeof value === "string" && value).join(" · ");
        if (summary) details.append(element("small", "", summary));
      }
      row.dataset.search = [name, record.type, record.protocol, record.server, record.listen, record.username].filter(value => typeof value === "string").join(" ").toLowerCase();
      row.hidden = !row.dataset.search.includes(query.trim().toLowerCase());
      if (typeof record.enabled === "boolean") details.append(localizedElement("small", "record-state", record.enabled ? "Enabled" : "Disabled"));
      row.append(details, operations);
      rows.append(row);
    });
  }

  const submit = button("Save draft", null, "primary");
  submit.type = "submit";
  controls.append(submit);
  if (isList) controls.append(button("Cancel", () => {
    if (dirty && !window.confirm(i18n.text("Discard unsaved changes?"))) return;
    dirty = false; edit.hidden = true;
  }));
  form.append(inputs, advanced, controls);
  form.onsubmit = event => {
    event.preventDefault();
    if (saving) return;
    let value;
    try { value = advanced.open ? jsonValue() : fieldValues(); }
    catch { notice.textContent = i18n.text("Enter a valid JSON object."); return; }
    if (!isList) { save(value); return; }
    const records = [...draft.value];
    if (index === null) records.push(value);
    else records[index] = value;
    save(records);
  };
  edit.append(label, form);
  if (isList) {
    drawList();
    edit.hidden = true;
    layout.append(list, edit);
  } else {
    start(draft.value, null);
    layout.append(edit);
  }
  return [localizedElement("p", "draft-label", "Saved drafts are independent of the running proxy Core."), layout];
}

async function open(key) {
  if (savingPage) return;
  if (dirty && !window.confirm(i18n.text("Discard unsaved changes?"))) return;
  dirty = false;
  const request = ++active;
  const page = serverPages.find(([id]) => id === key);
  i18n.bind(heading, page[1]);
  nav.querySelectorAll("button").forEach(item => {
    if (item.dataset.page === key) item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  });
  view.replaceChildren();
  notice.textContent = "";
  try {
    const command = key === "overview" || key === "system" ? { action: key } : { action: "load", section: key };
    const [value, profiles] = await Promise.all([
      send(command), ["nodes", "inbounds", "outbounds"].includes(key) ? send({ action: "security_profiles" }) : undefined
    ]);
    if (request !== active) return;
    if (key === "overview") view.append(...overview(value));
    else if (key === "system") view.append(panel("Host", infoRow("Operating system", value.os),
      infoRow("Architecture", value.architecture), infoRow("Logical CPUs", value.logical_cpus), infoRow("Panel version", value.version)));
    else {
      if (key === "services") view.append(panel("Core service", infoRow("Adapter", "Unavailable")));
      view.append(...editor(key, page[1], value, profiles));
    }
  } catch (error) {
    if (request === active) notice.textContent = i18n.message(error.message);
  }
}

let group;
for (const [key, title, icon, section] of serverPages) {
  if (group !== section) {
    group = section;
    nav.append(localizedElement("div", "nav-group", group));
  }
  const item = button("", () => open(key), "nav-item");
  item.dataset.page = key;
  const glyph = element("span", "nav-icon", icon);
  glyph.setAttribute("aria-hidden", "true");
  item.append(glyph, localizedElement("span", "", title));
  nav.append(item);
}

try {
  send = await transport("v2rayq-server");
  const savedTheme = await send({ action: "theme" });
  applyTheme(document.documentElement, savedTheme.theme);
  const themeLabel = element("label", "language-control appearance-control");
  themeLabel.append(localizedElement("span", "", "Theme"));
  const themeSelect = element("select");
  themeSelect.dataset.i18nLabel = "Theme";
  for (const [value, title] of Object.entries(themeChoices)) {
    const option = localizedElement("option", "", title); option.value = value; themeSelect.append(option);
  }
  themeSelect.value = document.documentElement.dataset.theme;
  themeSelect.onchange = async () => {
    themeSelect.disabled = true;
    try {
      await send({ action: "save_theme", theme: themeSelect.value });
      applyTheme(document.documentElement, themeSelect.value);
    } catch (error) { notice.textContent = i18n.message(error.message); }
    finally { themeSelect.disabled = false; themeSelect.value = document.documentElement.dataset.theme; }
  };
  themeLabel.append(themeSelect);
  document.querySelector(".rail").insertBefore(themeLabel, nav);
  const preference = await send({ action: "language" });
  await i18n.use(preference.language);
  const label = element("label", "language-control");
  label.append(localizedElement("span", "", "Language"));
  const choice = element("select");
  choice.dataset.i18nLabel = "Language";
  for (const item of i18n.languages) {
    const option = element("option", "", item.name);
    option.value = item.code;
    choice.append(option);
  }
  choice.value = i18n.code;
  choice.onchange = async () => {
    choice.disabled = true;
    try {
      await send({ action: "save_language", language: choice.value });
      await i18n.use(choice.value);
      updateLanguage();
    } catch (error) { notice.textContent = i18n.message(error.message); }
    finally { choice.disabled = false; choice.value = i18n.code; }
  };
  label.append(choice);
  document.querySelector(".rail").insertBefore(label, nav);
  document.querySelector(".brand small").dataset.i18n = "Server";
  updateLanguage();
  open("overview");
} catch (error) {
  notice.textContent = i18n.message(error.message);
}
