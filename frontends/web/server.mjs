import { transport } from "./transport.mjs";
import { fields, defaults } from "./server-fields.mjs";

const serverPages = [
  ["overview", "Overview", "⌂", "Status"],
  ["inbounds", "Inbounds", "⇥", "Endpoints"],
  ["users", "Users", "♙", "Endpoints"],
  ["outbounds", "Outbounds", "⇤", "Endpoints"],
  ["nodes", "Nodes", "◇", "Endpoints"],
  ["services", "Services", "▤", "System"],
  ["tls", "TLS", "♢", "System"],
  ["system", "System", "▧", "System"],
  ["routes", "Routes", "⇄", "Policies"],
  ["dns", "DNS", "◎", "Policies"],
  ["administrators", "Administrators", "♙", "Administration"],
  ["settings", "Settings", "⚙", "Administration"]
];
const nav = document.getElementById("navigation");
const heading = document.getElementById("heading");
const notice = document.getElementById("notice");
const view = document.getElementById("view");
let send;
let active = 0;

function element(tag, className = "", text) {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(label, action, className = "") {
  const node = element("button", className, label);
  node.type = "button";
  node.onclick = action;
  return node;
}

function panel(title, ...children) {
  const node = element("section", "panel");
  node.append(element("h2", "", title), ...children);
  return node;
}

function infoRow(name, value) {
  const row = element("div", "info-row");
  row.append(element("span", "", name), element("strong", "", String(value ?? "Unavailable")));
  return row;
}

function overview(status) {
  const topline = element("div", "metrics");
  for (const [key, label] of [["inbounds", "Inbounds"], ["users", "Users"], ["outbounds", "Outbounds"], ["nodes", "Nodes"]]) {
    const count = panel(label, element("strong", "metric", String(status.counts[key])));
    count.append(button("Open", () => open(key)));
    topline.append(count);
  }
  return [topline, panel("Deployment",
    infoRow("Core connection", status.core_connected ? "Connected" : "Awaiting adapter"),
    infoRow("Configuration", "Saved drafts"),
    infoRow("Administrative access", "Local launch session or Cockpit"))];
}

function editor(section, title, draft) {
  const isList = Array.isArray(draft.value);
  const layout = element("div", "editor-layout");
  const controls = element("div", "actions");
  const form = element("form", "editor-form");
  const inputs = element("div", "editor-fields");
  const advanced = element("details", "raw-editor");
  const textarea = element("textarea");
  textarea.setAttribute("aria-label", `${title} JSON`);
  textarea.spellcheck = false;
  advanced.append(element("summary", "", "Edit JSON"), textarea);
  const label = element("h2", "", "Edit configuration");
  const edit = element("section", "panel edit");
  const list = panel(title);
  let current = {};
  let index = null;
  let saving = false;

  function drawFields() {
    inputs.replaceChildren();
    for (const [key, title, kind] of fields[section]) {
      const value = current[key];
      const compatible = value === undefined || (kind === "list" && Array.isArray(value) && value.every(item => typeof item === "string"))
        || (kind === "port" && typeof value === "number") || (kind === "boolean" && typeof value === "boolean")
        || (!kind && typeof value === "string");
      if (!compatible) continue; // Keep structured or legacy values editable in the JSON view.
      const wrapper = element("label", "editor-field", title);
      const input = element("input");
      input.name = key;
      if (kind === "boolean") {
        input.type = "checkbox";
        input.checked = value === true;
        input.indeterminate = value === undefined;
      } else {
        input.type = kind === "port" ? "number" : "text";
        if (kind === "port") { input.min = "1"; input.max = "65535"; }
        if (kind === "list") input.placeholder = "Separate values with commas";
        input.value = Array.isArray(value) ? value.join(", ") : value ?? "";
      }
      input.oninput = () => { input.dataset.edited = "true"; };
      wrapper.append(input);
      inputs.append(wrapper);
    }
  }

  function fieldValues() {
    const value = { ...current };
    for (const [key, , kind] of fields[section]) {
      const input = inputs.querySelector(`[name="${key}"]`);
      if (!input || input.dataset.edited !== "true" || input.indeterminate) continue;
      if (kind === "boolean") value[key] = input.checked;
      else if (!input.value.trim()) delete value[key];
      else if (kind === "list") value[key] = input.value.split(",").map(item => item.trim()).filter(Boolean);
      else value[key] = kind === "port" ? input.valueAsNumber : input.value.trim();
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
        notice.textContent = "Enter a valid JSON object before returning to the form.";
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
    index = position;
    current = value;
    advanced.open = false;
    inputs.hidden = false;
    drawFields();
    label.textContent = position === null && isList ? "New record" : "Edit configuration";
    edit.hidden = false;
    if (isList) inputs.querySelector("input")?.focus();
  }

  async function save(value) {
    if (saving) return;
    saving = true;
    controls.querySelectorAll("button").forEach(control => { control.disabled = true; });
    try {
      const result = await send({ action: "save", section, revision: draft.revision, value });
      draft = { revision: result.revision, value };
      if (!layout.isConnected) return;
      notice.textContent = "Draft saved.";
      if (isList) { edit.hidden = true; drawList(); }
      else current = value;
    } catch (error) {
      if (layout.isConnected) notice.textContent = error.message;
    } finally {
      saving = false;
      controls.querySelectorAll("button").forEach(control => { control.disabled = false; });
    }
  }

  function drawList() {
    const header = element("div", "list-header");
    header.append(element("h2", "", `${draft.value.length} saved records`),
      button("Add record", () => start(defaults[section] || {}, null), "primary"));
    list.replaceChildren(header);
    draft.value.forEach((record, i) => {
      const row = element("div", "info-row");
      const candidate = record.name || record.tag || record.username || record.outbound;
      const name = typeof candidate === "string" && candidate ? candidate : `Record ${i + 1}`;
      const operations = element("div", "actions");
      operations.append(button("Edit", () => start(record, i)), button("Delete", () => {
        if (!saving && window.confirm(`Delete ${name}?`)) save(draft.value.filter((_, position) => position !== i));
      }, "danger"));
      row.append(element("strong", "", name), operations);
      list.append(row);
    });
  }

  const submit = button("Save draft", null, "primary");
  submit.type = "submit";
  controls.append(submit);
  if (isList) controls.append(button("Cancel", () => { edit.hidden = true; }));
  form.append(inputs, advanced, controls);
  form.onsubmit = event => {
    event.preventDefault();
    if (saving) return;
    let value;
    try { value = advanced.open ? jsonValue() : fieldValues(); }
    catch { notice.textContent = "Enter a valid JSON object."; return; }
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
  return [element("p", "draft-label", "Saved drafts are independent of the running proxy Core."), layout];
}

async function open(key) {
  const request = ++active;
  const page = serverPages.find(([id]) => id === key);
  heading.textContent = page[1];
  nav.querySelectorAll("button").forEach(item => {
    if (item.dataset.page === key) item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  });
  view.replaceChildren();
  notice.textContent = "";
  try {
    const value = await send(key === "overview" || key === "system" ? { action: key } : { action: "load", section: key });
    if (request !== active) return;
    if (key === "overview") view.append(...overview(value));
    else if (key === "system") view.append(panel("Host", infoRow("Operating system", value.os),
      infoRow("Architecture", value.architecture), infoRow("Logical CPUs", value.logical_cpus), infoRow("Panel version", value.version)));
    else {
      if (key === "services") view.append(panel("Core service", infoRow("Adapter", "Unavailable")));
      view.append(...editor(key, page[1], value));
    }
  } catch (error) {
    if (request === active) notice.textContent = error.message;
  }
}

let group;
for (const [key, title, icon, section] of serverPages) {
  if (group !== section) {
    group = section;
    nav.append(element("div", "nav-group", group));
  }
  const item = button("", () => open(key), "nav-item");
  item.dataset.page = key;
  const glyph = element("span", "nav-icon", icon);
  glyph.setAttribute("aria-hidden", "true");
  item.append(glyph, element("span", "", title));
  nav.append(item);
}

try {
  send = await transport("v2rayq-server");
  open("overview");
} catch (error) {
  notice.textContent = error.message;
}
