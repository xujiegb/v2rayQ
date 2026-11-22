const clientPages = [
  ["overview", "Overview", "⌂"],
  ["nodes", "Nodes", "◇"],
  ["routing", "Routing", "⇄"],
  ["settings", "Settings", "⚙"]
];
const protocols = { vless: "VLESS", vmess: "VMess", trojan: "Trojan", shadowsocks: "Shadowsocks", any_tls: "AnyTLS", hysteria2: "Hysteria2" };
const modes = { direct: "Direct", global: "Global", rules: "Rule-based" };
const services = { tun: "TUN", transparent: "Transparent proxy" };

function element(tag, className = "", text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(label, onClick, style = "") {
  const node = element("button", `vq-button ${style}`, label);
  node.type = "button";
  if (onClick) node.onclick = onClick;
  return node;
}

function field(label, name, options = {}) {
  const wrapper = element("label", "vq-field", label);
  const input = element(options.choices ? "select" : "input");
  input.name = name;
  if (options.choices) {
    for (const [value, title] of Object.entries(options.choices)) {
      const option = element("option", "", title);
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
  if (title) node.append(element("h2", "", title));
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
  if (!status?.installed) return "Service not installed";
  if (status.running === true) return "Running";
  if (status.running === false) return "Stopped";
  return "Status unavailable";
}

export function mountClient(root, send, { delegatedControl = false } = {}) {
  const state = { page: "overview", form: "", busy: false, profile: null, runtime: {} };
  const rail = element("aside", "vq-rail");
  const brand = element("div", "vq-brand");
  brand.append(element("span", "vq-mark", "◇"), element("strong", "", "v2rayQ"));
  const navigation = element("nav", "vq-nav");
  navigation.setAttribute("aria-label", "Client navigation");
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
    notice.textContent = "Working…";
    try {
      const response = await send(command);
      if (response.profile) state.profile = response.profile;
      if (response.runtime) state.runtime = response.runtime;
      notice.textContent = response.message || "";
      onSuccess?.();
      if (redraw) render();
    } catch (error) {
      notice.textContent = error.message || String(error);
    } finally {
      state.busy = false;
      root.setAttribute("aria-busy", "false");
    }
  }

  function navigate(page) {
    state.page = page;
    state.form = "";
    render();
  }

  function summary(profile) {
    const selected = profile.nodes.find(node => node.id === profile.selected_node);
    const hero = element("section", "vq-hero");
    hero.append(element("div", "vq-hero-kicker", "Selected node"),
      element("h2", "", selected?.name || "Choose a node"),
      element("p", "", selected ? `${protocols[selected.protocol]} · ${endpoint(selected)}` : "Import a share link or configure an endpoint."),
      button("Manage nodes", () => navigate("nodes"), "vq-light"));
    const grid = element("div", "vq-grid");
    grid.append(card("Profile", row("Nodes", String(profile.nodes.length)), row("Subscriptions", String(profile.subscriptions.length))),
      card("Network services", ...Object.entries(services).map(([key, title]) => row(title, serviceLabel(state.runtime[key])))));
    return [hero, grid];
  }

  function importForm() {
    const form = element("form", "vq-form");
    const submit = button("Save node", null, "vq-primary");
    submit.type = "submit";
    if (state.form === "share") {
      form.append(field("Share URI", "link", { type: "password", required: true }));
      submit.textContent = "Import node";
    } else if (state.form === "subscription") {
      form.append(field("Name", "name", { required: true }), field("HTTPS URL", "url", { type: "password", required: true }));
      submit.textContent = "Add and refresh";
    } else {
      const protocol = field("Protocol", "protocol", { choices: protocols });
      const credential = field("Credential", "credential", { type: "password" });
      const cipher = field("Cipher", "method", { value: "aes-128-gcm" });
      const sni = field("TLS server name", "server_name");
      const tls = field("TLS", "tls", { type: "checkbox" });
      const updateProtocol = () => {
        const kind = protocol.lastChild.value;
        const uuid = kind === "vless" || kind === "vmess";
        credential.firstChild.nodeValue = uuid ? "UUID" : "Password";
        credential.lastChild.required = kind !== "hysteria2";
        cipher.hidden = kind !== "shadowsocks";
        tls.hidden = !uuid;
        sni.hidden = kind === "shadowsocks" || (uuid && !tls.lastChild.checked);
      };
      protocol.onchange = updateProtocol;
      tls.onchange = updateProtocol;
      form.append(protocol, field("Name", "name", { required: true }), field("Server", "host", { required: true }),
        field("Port", "port", { value: "443", numeric: true, required: true }), credential, cipher, tls, sni);
      updateProtocol();
    }
    form.append(submit);
    form.onsubmit = event => {
      event.preventDefault();
      const values = Object.fromEntries(new FormData(form));
      const close = () => { state.form = ""; };
      if (state.form === "manual") {
        values.tls = values.tls === "on";
        perform({ action: "add_node", node: values }, close);
      } else {
        perform({ action: state.form === "share" ? "add_link" : "add_subscription", ...values }, close);
      }
    };
    return card("", form);
  }

  function endpoint(node) {
    return `${node.host.includes(":") ? `[${node.host}]` : node.host}:${node.port}`;
  }

  function nodeList(profile) {
    const actions = element("div", "vq-actions vq-node-actions");
    for (const [key, title] of [["share", "Import link"], ["manual", "Manual node"], ["subscription", "Add subscription"]]) {
      const item = button(title, () => { state.form = state.form === key ? "" : key; render(); }, state.form === key ? "vq-primary" : "");
      item.setAttribute("aria-pressed", String(state.form === key));
      actions.append(item);
    }
    const nodeRows = profile.nodes.map(node => {
      const selected = node.id === profile.selected_node;
      const select = button(selected ? "Selected" : "Select", () => perform({ action: "select_node", id: node.id }));
      select.disabled = selected;
      return row(node.name, `${protocols[node.protocol]} · ${endpoint(node)}`, select,
        button("Delete", () => perform({ action: "remove_node", id: node.id }), "vq-delete"));
    });
    const subscriptions = profile.subscriptions.map(item => row(item.name,
      item.last_error || (item.last_updated_at ? `Updated ${new Date(item.last_updated_at * 1000).toLocaleString()}` : "Awaiting refresh"),
      button("Refresh", () => perform({ action: "refresh_subscription", id: item.id })),
      button("Delete", () => {
        if (window.confirm("Delete this subscription and its imported nodes?")) perform({ action: "remove_subscription", id: item.id });
      }, "vq-delete")));
    return [actions, ...(state.form ? [importForm()] : []),
      card("Saved nodes", ...(nodeRows.length ? nodeRows : [element("p", "", "No nodes saved.")])),
      card("Subscriptions", ...(subscriptions.length ? subscriptions : [element("p", "", "No subscriptions saved.")]))];
  }

  function routing(settings) {
    const form = element("form", "vq-form");
    const save = button("Save routing", null, "vq-primary");
    save.type = "submit";
    form.append(element("p", "", "Installed proxy services use their own configuration."),
      field("Outbound policy", "route_mode", { choices: modes, value: settings.route_mode }),
      field("DNS resolver", "dns_server", { value: settings.dns_server }), save);
    form.onsubmit = event => {
      event.preventDefault();
      perform({ action: "save_routing", ...Object.fromEntries(new FormData(form)) }, null, false);
    };
    return [card("Profile defaults", form)];
  }

  function settings() {
    const rows = Object.entries(services).map(([mode, title]) => {
      const status = state.runtime[mode];
      const enabled = status?.running !== true;
      const toggle = button(enabled ? "Start" : "Stop", () => perform({ action: "set_network", mode, enabled }), "vq-primary");
      toggle.disabled = !(status?.can_change || (delegatedControl && status?.installed));
      return row(title, serviceLabel(status), toggle);
    });
    const authorization = delegatedControl ? "Service control uses the host session's permissions." : "On macOS, Linux and FreeBSD, graphical authorization uses SUDO_ASKPASS.";
    return [card("Traffic interception", ...rows, element("p", "", authorization),
      button("Refresh service status", () => perform({ action: "status" })))];
  }

  function render() {
    navigation.replaceChildren(...clientPages.map(([key, title, icon]) => {
      const item = button("", () => navigate(key), "vq-nav-item");
      item.append(element("span", "vq-icon", icon), element("span", "", title));
      item.firstChild.setAttribute("aria-hidden", "true");
      if (state.page === key) item.setAttribute("aria-current", "page");
      return item;
    }));
    view.replaceChildren(element("h1", "vq-heading", clientPages.find(([key]) => key === state.page)[1]));
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
