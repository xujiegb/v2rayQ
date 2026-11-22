import { mountClient } from "./client-ui.js";
import { transport } from "./transport.mjs";

const root = document.getElementById("client");
try {
  const cockpit = document.querySelector('meta[name="v2rayq-transport"]').content === "cockpit";
  mountClient(root, await transport("v2rayq-cli"), { delegatedControl: cockpit });
} catch (error) {
  root.textContent = error.message;
}
