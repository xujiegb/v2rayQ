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

function editor(section, title, draft, profiles = {}) {
  const isList = Array.isArray(draft.value);
  const layout = element("div", "editor-layout");
  const controls = element("div", "actions");
  const form = element("form", "editor-form");
  const inputs = element("fieldset", "editor-fields");
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
      if (parent === "transport" && !["vless", "vmess", "trojan"].includes(value("protocol"))) input.disabled = true;
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
      const wrapper = element("label", "editor-field", title);
      const select = choices[kind];
      const input = element(select ? "select" : "input");
      input.name = key;
      if (select) {
        input.setAttribute("aria-label", title);
        const values = new Map(Object.entries(select));
        if (value && !values.has(value)) values.set(value, value);
        const empty = element("option", "", "Select…");
        empty.value = "";
        input.append(empty);
        for (const [id, label] of values) {
          const option = element("option", "", label);
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
        if (kind === "list") input.placeholder = "Separate values with commas";
        if (kind === "profile") input.setAttribute("list", "security-profiles");
        input.value = Array.isArray(value) ? value.join(", ") : value ?? "";
      }
      input.oninput = () => { input.dataset.edited = "true"; if (select) refreshFields(); };
      wrapper.append(input);
      inputs.append(wrapper);
    }
    if (section === "nodes") {
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
      const details = element("div", "record-title");
      details.append(element("strong", "", name));
      if (section === "nodes") {
        const endpoint = typeof record.server === "string" && record.server_port ? `${record.server}:${record.server_port}` : record.server;
        const protocol = record.protocol || record.type;
        const summary = [choices.protocol[protocol] || protocol, endpoint, choices.security[record.security]]
          .filter(value => typeof value === "string" && value).join(" · ");
        if (summary) details.append(element("small", "", summary));
      }
      row.append(details, operations);
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
    const command = key === "overview" || key === "system" ? { action: key } : { action: "load", section: key };
    const [value, profiles] = await Promise.all([
      send(command), key === "nodes" ? send({ action: "security_profiles" }) : undefined
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
