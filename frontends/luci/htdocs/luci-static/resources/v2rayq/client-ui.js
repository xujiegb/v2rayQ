import { loadLocalization } from "./i18n.js";

const clientPages = [
  ["overview", "Overview", "⌂"],
  ["nodes", "Nodes", "◇"],
  ["routing", "Routing", "⇄"],
  ["settings", "Settings", "⚙"]
];
const protocols = { vless: "VLESS", vmess: "VMess", trojan: "Trojan", shadowsocks: "Shadowsocks", any_tls: "AnyTLS", hysteria2: "Hysteria2" };
const transports = { tcp: "TCP", web_socket: "WebSocket", http: "HTTP", http_upgrade: "HTTP Upgrade", grpc: "gRPC" };
const modes = { direct: "Direct", global: "Global", rules: "Rule-based" };
const ruleMatches = { domain: "Domain", domain_suffix: "Domain suffix", ip_cidr: "IP / CIDR", port: "Destination port" };
const ruleTargets = { proxy: "Selected proxy", direct: "Direct", block: "Block" };
const services = { tun: "TUN", transparent: "Transparent proxy" };

export async function mountClient(root, send, { delegatedControl = false } = {}) {
  const i18n = await loadLocalization(new URL("./locales/", import.meta.url), "client");
  function element(tag, className = "", text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  function button(label, onClick, style = "") {
    const node = element("button", `vq-button ${style}`, i18n.text(label));
    node.type = "button";
    if (onClick) node.onclick = onClick;
    return node;
  }

  function field(label, name, options = {}) {
    const wrapper = element("label", "vq-field", i18n.text(label));
    const input = element(options.choices ? "select" : "input");
    input.name = name;
    if (options.choices) {
      input.setAttribute("aria-label", i18n.text(label));
      for (const [value, title] of Object.entries(options.choices)) {
        const option = element("option", "", i18n.text(title));
        option.value = value;
        input.append(option);
      }
    } else {
      input.type = options.type || "text";
      input.required = options.required || false;
      input.autocomplete = "off";
      if (options.numeric) input.inputMode = "numeric";
    }
    if (options.value !== undefined) input.value = options.value;
    wrapper.append(input);
    return wrapper;
  }

  function card(title, ...children) {
    const node = element("section", "vq-card");
    if (title) node.append(element("h2", "", i18n.text(title)));
    node.append(...children);
    return node;
  }

  function row(title, detail, ...actions) {
    const layout = element("div", "vq-row");
    const body = element("div", "vq-row-content");
    body.append(element("strong", "", title), element("small", "", detail));
    const controls = element("div", "vq-actions");
    controls.append(...actions);
    layout.append(body, controls);
    return layout;
  }

  function serviceLabel(status) {
    if (!status?.installed) return i18n.text("Service not installed");
    if (status.running === true) return i18n.text("Running");
    if (status.running === false) return i18n.text("Stopped");
    return i18n.text("Status unavailable");
  }

  const state = {
    page: "overview", form: "", busy: false, profile: null, runtime: {}, query: "", nodePage: 0,
    editingNode: null, editingSubscription: null, editingRule: undefined, routingDraft: null, configuration: "", usePreferences: false
  };
  const rail = element("aside", "vq-rail");
  const brand = element("div", "vq-brand");
  brand.append(element("span", "vq-mark", "◇"), element("strong", "", "v2rayQ"));
  const navigation = element("nav", "vq-nav");
  navigation.setAttribute("aria-label", i18n.text("Client navigation"));
  const content = element("div", "vq-content");
  const notice = element("div", "vq-notice");
  notice.setAttribute("role", "status");
  const view = element("div", "vq-view");
  rail.append(brand, navigation);
  content.append(notice, view);
  root.replaceChildren(rail, content);

  async function perform(command, onSuccess, redraw = true) {
    if (state.busy) return;
    state.busy = true;
    root.setAttribute("aria-busy", "true");
    notice.textContent = i18n.text("Working…");
    try {
      const response = await send(command);
      if (response.profile) {
        state.profile = response.profile;
        await i18n.use(response.profile.settings.language);
      }
      if (response.runtime) state.runtime = response.runtime;
      notice.textContent = i18n.message(response.message || "");
      if (response.merge) notice.textContent = i18n.format("{nodes} nodes · {subscriptions} subscriptions · {rules} rules · {duplicates} duplicates", response.merge);
      onSuccess?.(response);
      if (redraw) render();
    } catch (error) {
      notice.textContent = i18n.message(error.message || String(error));
    } finally {
      state.busy = false;
      root.setAttribute("aria-busy", "false");
    }
  }

  function navigate(page) {
    state.page = page;
    state.form = "";
    state.editingNode = null;
    state.editingSubscription = null;
    state.editingRule = undefined;
    render();
  }

  function summary(profile) {
    const selected = profile.nodes.find(node => node.id === profile.selected_node);
    const hero = element("section", "vq-hero");
    hero.append(element("div", "vq-hero-kicker", i18n.text("Selected node")),
      element("h2", "", selected?.name || i18n.text("Choose a node")),
      element("p", "", selected ? `${protocols[selected.protocol]} · ${endpoint(selected)}` : i18n.text("Import a share link or configure an endpoint.")),
      button("Manage nodes", () => navigate("nodes"), "vq-light"));
    const grid = element("div", "vq-grid");
    grid.append(card("Profile", row(i18n.text("Nodes"), String(profile.nodes.length)), row(i18n.text("Subscriptions"), String(profile.subscriptions.length))),
      card("Network services", ...Object.entries(services).map(([key, title]) => row(i18n.text(title), serviceLabel(state.runtime[key])))));
    return [hero, grid];
  }

  function shareForm() {
    const form = element("form", "vq-import-form");
    const label = element("label", "vq-field", i18n.text("Share links or base64 list"));
    const text = element("textarea");
    text.required = true;
    text.rows = 6;
    text.spellcheck = false;
    text.autocomplete = "off";
    text.placeholder = i18n.text("Paste one link per line or select a text file");
    label.append(text);
    const source = element("label", "vq-field vq-file", i18n.text("Import from file"));
    const file = element("input");
    file.type = "file";
    file.accept = ".txt,.list,.b64,text/plain";
    source.append(file);
    file.onchange = async () => {
      const item = file.files?.[0];
      if (!item) return;
      if (item.size > 96 * 1024) { notice.textContent = i18n.text("Select a text file of up to 96 KiB."); return; }
      try {
        text.value = new TextDecoder("utf-8", { fatal: true }).decode(await item.arrayBuffer());
        notice.textContent = i18n.text("File ready to preview.");
      } catch {
        notice.textContent = i18n.text("Could not read the file as UTF-8 text.");
      }
    };
    const actions = element("div", "vq-actions");
    const submit = button("Import nodes", null, "vq-primary");
    submit.type = "submit";
    actions.append(button("Preview import", () => {
      if (form.reportValidity()) perform({ action: "preview_import", content: text.value }, null, false);
    }), submit);
    form.append(label, source, actions);
    form.onsubmit = event => {
      event.preventDefault();
      perform({ action: "import_nodes", content: text.value }, () => { state.form = ""; });
    };
    return card("Import share links", form);
  }

  function subscriptionForm(subscription) {
    const form = element("form", "vq-form");
    const submit = button(subscription ? "Save subscription" : "Add and refresh", null, "vq-primary");
    submit.type = "submit";
    form.append(field("Name (optional)", "name", { value: subscription?.name || "" }),
      field(subscription ? "New HTTPS URL (leave blank to keep current URL)" : "HTTPS URL", "url",
        { type: "password", required: !subscription }), submit);
    form.onsubmit = event => {
      event.preventDefault();
      const values = Object.fromEntries(new FormData(form));
      const command = subscription
        ? { action: "update_subscription", id: subscription.id, ...values }
        : { action: "add_subscription", ...values };
      perform(command, () => { state.form = ""; state.editingSubscription = null; });
    };
    const editor = card(subscription ? "Edit subscription" : "New subscription", form);
    editor.dataset.editor = "subscription";
    return editor;
  }

  function manualForm() {
    const form = element("form", "vq-form");
    const submit = button("Save node", null, "vq-primary");
    submit.type = "submit";
    const protocol = field("Protocol", "protocol", { choices: protocols });
    const credential = field("Credential", "credential", { type: "password" });
    const cipher = field("Cipher", "method", { value: "aes-128-gcm" });
    const sni = field("TLS server name", "server_name");
    const tls = field("TLS", "tls", { type: "checkbox" });
    const transport = field("Transport", "transport", { choices: transports });
    const transportPath = field("Transport path", "transport_path");
    const transportHost = field("Transport host", "transport_host");
    const serviceName = field("gRPC service name", "service_name");
    const alpn = field("ALPN (comma separated)", "alpn");
    const advanced = element("details", "vq-advanced");
    const options = element("div", "vq-form");
    options.append(transport, transportPath, transportHost, serviceName, alpn);
    advanced.append(element("summary", "", i18n.text("Advanced connection options")), options);
    const updateProtocol = () => {
      const kind = protocol.lastChild.value;
      const uuid = kind === "vless" || kind === "vmess";
      credential.firstChild.nodeValue = i18n.text(uuid ? "UUID" : "Password");
      credential.lastChild.required = kind !== "hysteria2";
      cipher.hidden = kind !== "shadowsocks";
      tls.hidden = !uuid;
      const transportVisible = uuid || kind === "trojan";
      const transportKind = transport.lastChild.value;
      sni.hidden = kind === "shadowsocks" || (uuid && !tls.lastChild.checked);
      advanced.hidden = kind === "shadowsocks";
      transport.hidden = !transportVisible;
      transportPath.hidden = transportHost.hidden = !transportVisible || transportKind === "tcp" || transportKind === "grpc";
      serviceName.hidden = !transportVisible || transportKind !== "grpc";
      alpn.hidden = sni.hidden;
    };
    protocol.onchange = updateProtocol;
    tls.onchange = updateProtocol;
    transport.onchange = updateProtocol;
    form.append(protocol, field("Name", "name", { required: true }), field("Server", "host", { required: true }),
      field("Port", "port", { value: "443", numeric: true, required: true }), credential, cipher, tls, sni, advanced);
    updateProtocol();
    form.append(submit);
    form.onsubmit = event => {
      event.preventDefault();
      const values = Object.fromEntries(new FormData(form));
      values.tls = values.tls === "on";
      perform({ action: "add_node", node: values }, () => { state.form = ""; });
    };
    return card("Manual node", form);
  }

  function endpoint(node) {
    return `${node.host.includes(":") ? `[${node.host}]` : node.host}:${node.port}`;
  }

  function endpointForm(node) {
    const form = element("form", "vq-form");
    const actions = element("div", "vq-actions");
    const save = button("Save endpoint", null, "vq-primary");
    save.type = "submit";
    actions.append(save, button("Cancel", () => { state.editingNode = null; render(); }));
    form.append(field("Name", "name", { value: node.name, required: true }),
      field("Server", "host", { value: node.host, required: true }),
      field("Port", "port", { value: String(node.port), required: true, numeric: true }), actions);
    form.onsubmit = event => {
      event.preventDefault();
      perform({ action: "update_node", id: node.id, ...Object.fromEntries(new FormData(form)) },
        () => { state.editingNode = null; });
    };
    const editor = card("Edit node endpoint", form);
    editor.dataset.editor = "endpoint";
    return editor;
  }

  function openNodeEditor(id) {
    state.editingNode = id;
    state.editingSubscription = null;
    state.form = "";
    render();
    view.querySelector('[data-editor="endpoint"] input')?.focus();
  }

  function nodeList(profile) {
    const actions = element("div", "vq-actions vq-node-actions");
    for (const [key, title] of [["share", "Import links"], ["manual", "Manual node"], ["subscription", "Add subscription"]]) {
      const item = button(title, () => {
        state.form = state.form === key ? "" : key;
        state.editingNode = null;
        state.editingSubscription = null;
        render();
      }, state.form === key ? "vq-primary" : "");
      item.setAttribute("aria-pressed", String(state.form === key));
      actions.append(item);
    }
    const search = field("Search nodes", "node-search", { type: "search", value: state.query });
    const nodeRows = element("div");
    const pagination = element("div", "vq-pagination");
    pagination.setAttribute("aria-label", i18n.text("Node pages"));
    const pageSize = 30;
    function showNodes() {
      const query = state.query.trim().toLowerCase();
      const matches = query ? profile.nodes.filter(node =>
        [node.name, endpoint(node), protocols[node.protocol]].some(value => value.toLowerCase().includes(query))) : profile.nodes;
      const totalPages = Math.max(1, Math.ceil(matches.length / pageSize));
      state.nodePage = Math.min(state.nodePage, totalPages - 1);
      const start = state.nodePage * pageSize;
      nodeRows.replaceChildren(...matches.slice(start, start + pageSize).map(node => {
        const selected = node.id === profile.selected_node;
        const select = button(selected ? "Selected" : "Select", () => perform({ action: "select_node", id: node.id }));
        select.disabled = selected;
        const source = profile.subscriptions.find(item => item.id === node.subscription_id)?.name;
        const detail = `${protocols[node.protocol]} · ${endpoint(node)}${source ? ` · ${source}` : ""}`;
        const edit = node.subscription_id
          ? button("Make local copy", () => perform({ action: "copy_node", id: node.id }))
          : button("Edit endpoint", () => openNodeEditor(node.id));
        return row(node.name, detail, select, edit,
          button("Delete", () => perform({ action: "remove_node", id: node.id }, () => {
            if (state.editingNode === node.id) state.editingNode = null;
          }), "vq-delete"));
      }));
      if (!matches.length) nodeRows.append(element("p", "", i18n.text(query ? "No matching nodes." : "No nodes saved.")));
      const nodeCount = i18n.format(matches.length === 1 ? "{count} node" : "{count} nodes", { count: matches.length });
      const label = totalPages > 1 ? `${nodeCount} · ${state.nodePage + 1} / ${totalPages}` : nodeCount;
      const count = element("small", "", label);
      count.setAttribute("role", "status");
      pagination.replaceChildren(count);
      if (totalPages > 1) {
        const previous = button("Previous", () => { state.nodePage--; showNodes(); });
        const next = button("Next", () => { state.nodePage++; showNodes(); });
        previous.disabled = state.nodePage === 0;
        next.disabled = state.nodePage + 1 >= totalPages;
        pagination.append(previous, next);
      }
    }
    search.lastChild.addEventListener("input", event => {
      state.query = event.target.value;
      state.nodePage = 0;
      showNodes();
    });
    showNodes();
    const subscriptions = profile.subscriptions.map(item => {
      const count = i18n.format(item.node_count === 1 ? "{count} node" : "{count} nodes", { count: item.node_count });
      const status = (item.last_error ? i18n.message(item.last_error) : "") || (item.last_updated_at
        ? i18n.format("Updated {date}", { date: new Date(item.last_updated_at * 1000).toLocaleString() }) : i18n.text("Awaiting refresh"));
      return row(item.name, `${count} · ${status}`,
        button("Edit subscription", () => {
          state.editingSubscription = item.id;
          state.editingNode = null;
          state.form = "subscription";
          render();
          view.querySelector('[data-editor="subscription"] input')?.focus();
        }),
        button("Refresh", () => perform({ action: "refresh_subscription", id: item.id })),
        button("Delete", () => {
          if (window.confirm(i18n.text("Delete this subscription and its imported nodes?"))) perform({ action: "remove_subscription", id: item.id }, () => {
            if (state.editingSubscription === item.id) { state.editingSubscription = null; state.form = ""; }
          });
        }, "vq-delete"));
    });
    const edited = profile.nodes.find(node => node.id === state.editingNode);
    const subscription = profile.subscriptions.find(item => item.id === state.editingSubscription);
    const forms = { share: shareForm, subscription: () => subscriptionForm(subscription), manual: manualForm };
    return [actions, ...(state.form ? [forms[state.form]()] : []), ...(edited ? [endpointForm(edited)] : []),
      card("Saved nodes", search, nodeRows, pagination),
      card("Subscriptions", ...(subscriptions.length ? subscriptions : [element("p", "", i18n.text("No subscriptions saved."))]))];
  }

  function ruleEditor(rule) {
    const form = element("form", "vq-form vq-rule-editor");
    form.dataset.editor = "rule";
    const actions = element("div", "vq-actions");
    const submit = button("Save rule", null, "vq-primary");
    submit.type = "submit";
    actions.append(submit, button("Cancel", () => { state.editingRule = undefined; render(); }));
    form.append(field("Match on", "kind", { choices: ruleMatches, value: rule?.kind || "domain" }),
      field("Value", "value", { value: rule?.value || "", required: true }),
      field("Action", "target", { choices: ruleTargets, value: rule?.target || "proxy" }), actions);
    form.onsubmit = event => {
      event.preventDefault();
      const values = Object.fromEntries(new FormData(form));
      perform({ action: "save_route_rule", id: rule?.id || null, rule: values }, () => { state.editingRule = undefined; });
    };
    return form;
  }

  function showRuleEditor(id) {
    state.editingRule = id;
    render();
    view.querySelector('[data-editor="rule"] select')?.focus();
  }

  function routing(settings) {
    const form = element("form", "vq-form");
    const save = button("Save routing", null, "vq-primary");
    save.type = "submit";
    const draft = state.routingDraft || settings;
    form.append(field("Outbound policy", "route_mode", { choices: modes, value: draft.route_mode }),
      field("DNS resolver", "dns_server", { value: draft.dns_server }), save);
    form.oninput = () => { state.routingDraft = Object.fromEntries(new FormData(form)); };
    form.onsubmit = event => {
      event.preventDefault();
      perform({ action: "save_routing", ...Object.fromEntries(new FormData(form)) }, () => { state.routingDraft = null; }, false);
    };
    const rules = settings.route_rules || [];
    const header = element("div", "vq-card-header");
    header.append(element("h2", "", i18n.text("Route rules")), button("Add rule", () => showRuleEditor(null), "vq-primary"));
    const rows = rules.map((rule, index) => {
      const move = direction => perform({ action: "move_route_rule", id: rule.id, direction });
      const up = button("Move up", () => move("up"));
      const down = button("Move down", () => move("down"));
      up.disabled = index === 0;
      down.disabled = index === rules.length - 1;
      return row(`${index + 1} · ${rule.value}`, `${i18n.text(ruleMatches[rule.kind])} → ${i18n.text(ruleTargets[rule.target])}`,
        button("Edit rule", () => showRuleEditor(rule.id)), up, down,
        button("Delete rule", () => perform({ action: "remove_route_rule", id: rule.id }, () => {
          if (state.editingRule === rule.id) state.editingRule = undefined;
        }), "vq-delete"));
    });
    const rulesCard = card("", header, element("p", "", i18n.text("Order is top to bottom. Rules are saved locally for the future Core.")));
    if (state.editingRule !== undefined) rulesCard.append(ruleEditor(rules.find(rule => rule.id === state.editingRule)));
    rulesCard.append(...(rows.length ? rows : [element("p", "", i18n.text("No route rules saved."))]));
    return [card("Profile defaults", form), rulesCard];
  }

  function configurationSettings() {
    const form = element("div", "vq-import-form");
    const textLabel = element("label", "vq-field", i18n.text("Configuration JSON"));
    const text = element("textarea");
    text.rows = 6;
    text.spellcheck = false;
    text.autocomplete = "off";
    text.value = state.configuration;
    text.oninput = () => { state.configuration = text.value; };
    textLabel.append(text);
    const fileLabel = element("label", "vq-field vq-file", i18n.text("Choose configuration file"));
    const file = element("input");
    file.type = "file";
    file.accept = ".json,application/json";
    fileLabel.append(file);
    file.onchange = async () => {
      const item = file.files?.[0];
      if (!item) return;
      if (item.size > 4 * 1024 * 1024) { notice.textContent = i18n.text("Configuration exceeds 4 MiB"); return; }
      try {
        state.configuration = new TextDecoder("utf-8", { fatal: true }).decode(await item.arrayBuffer());
        text.value = state.configuration;
      } catch { notice.textContent = i18n.text("Could not read configuration file"); }
    };
    const preferences = field("Use imported preferences", "preferences", { type: "checkbox" });
    preferences.classList.add("vq-check");
    preferences.lastChild.checked = state.usePreferences;
    preferences.onchange = () => { state.usePreferences = preferences.lastChild.checked; };
    const actions = element("div", "vq-actions");
    actions.append(button("Download configuration", () => perform({ action: "export_configuration" }, response => {
      const url = URL.createObjectURL(new Blob([response.configuration], { type: "application/json" }));
      const link = document.createElement("a");
      link.href = url;
      link.download = "v2rayq-client.json";
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      notice.textContent = i18n.text("Configuration exported");
    }, false)), button("Preview merge", () => perform({ action: "merge_configuration", content: state.configuration,
      use_preferences: state.usePreferences, preview: true }, null, false)),
      button("Merge configuration", () => perform({ action: "merge_configuration", content: state.configuration,
        use_preferences: state.usePreferences }, () => { state.configuration = ""; }), "vq-primary"));
    form.append(element("p", "", i18n.text("The export includes node credentials and subscription URLs. Keep the file private.")),
      fileLabel, textLabel, preferences, actions);
    return card("Configuration export and merge", form);
  }

  function settings() {
    const rows = Object.entries(services).map(([mode, title]) => {
      const status = state.runtime[mode];
      const enabled = status?.running !== true;
      const toggle = button(enabled ? "Start" : "Stop", () => perform({ action: "set_network", mode, enabled }), "vq-primary");
      toggle.disabled = !(status?.can_change || (delegatedControl && status?.installed));
      return row(i18n.text(title), serviceLabel(status), toggle);
    });
    const authorization = delegatedControl ? "Service control uses the host session's permissions." : "On macOS, Linux and FreeBSD, graphical authorization uses SUDO_ASKPASS.";
    const language = field("Language", "language", { choices: Object.fromEntries(i18n.languages.map(item => [item.code, item.name])), value: i18n.code });
    language.lastChild.onchange = () => perform({ action: "save_language", language: language.lastChild.value });
    return [card("Appearance", language), configurationSettings(), card("Traffic interception", ...rows, element("p", "", i18n.text(authorization)),
      button("Refresh service status", () => perform({ action: "status" })))];
  }

  function render() {
    navigation.setAttribute("aria-label", i18n.text("Client navigation"));
    navigation.replaceChildren(...clientPages.map(([key, title, icon]) => {
      const item = button("", () => navigate(key), "vq-nav-item");
      item.append(element("span", "vq-icon", icon), element("span", "", i18n.text(title)));
      item.firstChild.setAttribute("aria-hidden", "true");
      if (state.page === key) item.setAttribute("aria-current", "page");
      return item;
    }));
    view.replaceChildren(element("h1", "vq-heading", i18n.text(clientPages.find(([key]) => key === state.page)[1])));
    const profile = state.profile;
    if (!profile) {
      view.append(button("Load profile", async () => {
        await perform({ action: "profile" });
        if (state.profile) await perform({ action: "status" });
      }, "vq-primary"));
      return;
    }
    const pages = { overview: () => summary(profile), nodes: () => nodeList(profile), routing: () => routing(profile.settings), settings };
    view.append(...pages[state.page]());
  }

  render();
  perform({ action: "profile" }).then(() => { if (state.profile) perform({ action: "status" }); });
}
