function loadCockpit() {
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "../base1/cockpit.js";
    script.onload = () => resolve(window.cockpit);
    script.onerror = () => reject(new Error("Cockpit bridge is unavailable."));
    document.head.append(script);
  });
}

export async function transport(binary) {
  if (document.querySelector('meta[name="v2rayq-transport"]')?.content === "cockpit") {
    const cockpit = await loadCockpit();
    return async command => {
      const privileged = command.action === "set_network";
      const entry = privileged ? "--service-rpc" : "--rpc";
      const process = cockpit.spawn([`/usr/bin/${binary}`, entry], privileged ? { superuser: "require" } : {});
      process.input(JSON.stringify(command));
      return result(JSON.parse(await process));
    };
  }
  const fragment = new URLSearchParams(location.hash.slice(1));
  const token = fragment.get("access") || sessionStorage.getItem("v2rayq-access");
  if (fragment.has("access")) {
    sessionStorage.setItem("v2rayq-access", token);
    history.replaceState(null, "", location.pathname + location.search);
  }
  return async command => {
    if (!token) throw new Error("Open the access URL printed when v2rayQ starts.");
    const response = await fetch("/api/v1/command", {
      method: "POST", credentials: "omit", cache: "no-store",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
      body: JSON.stringify(command)
    });
    if (response.status === 403) throw new Error("Access expired. Open the current launch URL.");
    if (!response.ok) throw new Error(`Request failed (${response.status}).`);
    return result(await response.json());
  };
}

export function result(response) {
  if (response.error) throw new Error(response.error);
  return response.result;
}
