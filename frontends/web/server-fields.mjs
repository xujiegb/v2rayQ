// These fields belong to panel drafts. The Core adapter defines its own schema.
export const fields = {
  inbounds: [
    ["tag", "Name"], ["type", "Protocol"], ["listen", "Listen address"], ["listen_port", "Listen port", "port"]
  ],
  users: [["name", "Name"], ["enabled", "Enabled", "boolean"]],
  outbounds: [["tag", "Name"], ["type", "Protocol"], ["server", "Server"], ["server_port", "Server port", "port"]],
  nodes: [["name", "Name"], ["server", "Server"], ["server_port", "Server port", "port"]],
  services: [["name", "Name"], ["enabled", "Enabled", "boolean"]],
  tls: [["name", "Name"], ["certificate_path", "Certificate path"], ["key_path", "Private key path"]],
  routes: [["name", "Name"], ["domain_suffix", "Domain suffixes", "list"], ["outbound", "Outbound name"]],
  dns: [["servers", "Resolvers", "list"], ["strategy", "Resolution strategy"]],
  administrators: [["username", "Username"], ["role", "Role"]],
  settings: [["name", "Panel name"], ["listen", "Listen address"], ["listen_port", "Listen port", "port"]]
};

export const defaults = {
  inbounds: { listen: "::", listen_port: 443 }, users: { enabled: true },
  nodes: { server_port: 443 }, services: { enabled: false }, routes: { domain_suffix: [] },
  administrators: { role: "viewer" }
};
