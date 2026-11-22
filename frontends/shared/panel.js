"use strict";

// Static UI definitions only: never imply these controls reach a real Core.
const serverPages = [
  ["Overview","⌂","Health, resources and connection summary."],
  ["Inbounds","⇥","Inbound listeners and authentication settings."],
  ["Users","♙","User accounts and access limits."],
  ["Outbounds","⇤","Proxy destinations and outbound policies."],
  ["Nodes","◈","Node registry and endpoint management."],
  ["Services","▤","Core service lifecycle and diagnostics."],
  ["TLS","♢","Certificates and TLS settings."],
  ["System","▧","Host information and runtime details."],
  ["Routes","⇄","Routing rules and rule-set priorities."],
  ["DNS","◎","Resolvers and DNS strategy."],
  ["Administrators","♙","Administrative accounts and audit permissions."],
  ["Settings","⚙","Panel configuration and security."]
];
const clientPages = [
  ["Overview","⌂","Connection health and current traffic."],
  ["Nodes","◈","Manual nodes and HTTPS subscriptions."],
  ["Routing","⇄","Direct, global and rules-based routing."],
  ["Settings","⚙","System proxy, TUN and appearance."]
];
const isServer = document.body.dataset.panel === "server";
const pages = isServer ? serverPages : clientPages;
const navigation = document.getElementById("navigation");
const view = document.getElementById("view");
const heading = document.getElementById("page-heading");
const description = document.getElementById("page-description");
const breadcrumb = document.getElementById("breadcrumb");

function el(tag, className, text) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

function card(title, value, note) {
  const element = el("section", "card");
  element.append(el("div","metric-label",title),el("div","metric-value",value),el("div","metric-note",note));
  return element;
}

function moduleCard(item) {
  const element = el("section","module-card");
  element.append(el("div","symbol",item[1]),el("strong","",item[0]),el("p","",item[2]));
  return element;
}

function statusRows() {
  const element = el("section", "card");
  element.append(el("h2","section-title","Workspace status"));
  const rows = [
    ["Protocol", "API v1"],
    ["Core", "Not connected"],
    ["Operation mode", "Read-only preview"],
    ["Authentication", "Not configured"],
    ["Remote administration", "Disabled"]
  ];
  rows.forEach(([name,value]) => {
    const row = el("div","info-row");
    row.append(el("span","",name),el("strong","",value));
    element.append(row);
  });
  return element;
}

function emptyState(title, message, icon) {
  const element = el("div","empty-state");
  element.append(el("span","empty-icon",icon),el("h3","",title),el("p","",message));
  return element;
}

function showPage(index) {
  const item = pages[index] || pages[0];
  const title = item[0];
  heading.textContent = title;
  breadcrumb.textContent = title;
  description.textContent = item[2];
  view.replaceChildren();
  navigation.querySelectorAll("button").forEach((button, i) => {
    button.classList.toggle("active", i === index);
    button.setAttribute("aria-current", i === index ? "page" : "false");
  });
  if (index === 0) {
    const metrics = el("div","metrics");
    const values = isServer
      ? [["CORE","Offline","No API connected"],["INBOUNDS","—","Awaiting adapter"],["USERS","—","Awaiting adapter"],["TRAFFIC","—","No samples"]]
      : [["CONNECTION","Offline","Awaiting adapter"],["NODES","—","Open the native client"],["POLICY","—","Not applied"],["TRAFFIC","—","No samples"]];
    values.forEach(([label,value,note]) => metrics.append(card(label,value,note)));
    const grid = el("div","card-grid");
    const first = el("section","card");
    first.append(el("h2","section-title",isServer ? "Core activity" : "Connection activity"),
      emptyState("Nothing is running yet","The Rust Core management interface has not been published. This panel does not fabricate traffic or live status.","◈"));
    grid.append(first,statusRows());
    view.append(metrics,grid);
  } else {
    view.append(el("div","notice","This page is a UI foundation. Editing and system operations will be enabled only after an authenticated Core adapter is implemented."));
    const grid = el("div","module-cards");
    const sections = isServer
      ? [["Configuration","Field models and validation"],["Operations","Requires Core integration"],["Security","Requires role-based API authorization"]]
      : [["Local profile","Use egui to manage nodes now"],["Connectivity","Connect a backend later"],["System permissions","TUN requires OS authorization"]];
    sections.forEach(([name,detail]) => grid.append(moduleCard([name,item[1],detail])));
    view.append(grid);
  }
}

pages.forEach((item,index) => {
  const button = el("button","nav-item");
  button.type = "button";
  button.append(el("span","nav-icon",item[1]),el("span","",item[0]));
  button.addEventListener("click", () => showPage(index));
  navigation.append(button);
});
showPage(0);
