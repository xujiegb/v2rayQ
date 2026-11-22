// These fields belong to panel drafts. The Core adapter defines its own schema.
export const fields = {
  inbounds: [
    ["tag", "Name"], ["type", "Protocol", "protocol"], ["listen", "Listen address"], ["listen_port", "Listen port", "port"],
    ["security", "Security", "security"], ["security_profile", "Security profile name", "profile", ["security", ["tls", "reality"]]]
  ],
  users: [["name", "Name"], ["enabled", "Enabled", "boolean"], ["inbound", "Inbound name"], ["credential", "Credential", "secret"]],
  outbounds: [["tag", "Name"], ["type", "Protocol", "protocol"], ["server", "Server"], ["server_port", "Server port", "port"], ["node", "Node name"],
    ["security", "Security", "security"], ["security_profile", "Security profile name", "profile", ["security", ["tls", "reality"]]]],
  nodes: [
    ["name", "Name"], ["protocol", "Protocol", "protocol"], ["server", "Server"], ["server_port", "Server port", "port"],
    ["uuid", "UUID", "text", ["protocol", ["vless", "vmess"]]],
    ["password", "Password", "secret", ["protocol", ["trojan", "shadowsocks", "anytls", "hysteria2"]]],
    ["cipher", "Cipher", "text", ["protocol", ["shadowsocks"]]],
    ["transport", "Transport", "transport", ["protocol", ["vless", "vmess", "trojan"]]],
    ["path", "Transport path", "text", ["transport", ["ws", "http", "http_upgrade"]]],
    ["service_name", "gRPC service name", "text", ["transport", ["grpc"]]],
    ["security", "Security", "security"],
    ["security_profile", "Security profile name", "profile", ["security", ["tls", "reality"]]]
  ],
  services: [["name", "Name"], ["enabled", "Enabled", "boolean"]],
  tls: [["name", "Name"], ["server_name", "Server name"], ["certificate_path", "Certificate path"],
    ["key_path", "Private key path"], ["alpn", "ALPN", "list"]],
  reality: [["name", "Name"], ["server_names", "Server names", "list"], ["handshake_host", "Handshake target"],
    ["handshake_port", "Handshake port", "port"], ["private_key", "Private key", "secret"],
    ["public_key", "Public key"], ["short_ids", "Short IDs", "list"]],
  routes: [["name", "Name"], ["domain_suffix", "Domain suffixes", "list"], ["outbound", "Outbound name"]],
  dns: [["servers", "Resolvers", "list"], ["strategy", "Resolution strategy"]],
  administrators: [["username", "Username"], ["role", "Role"]],
  settings: [["name", "Panel name"], ["listen", "Listen address"], ["listen_port", "Listen port", "port"]]
};

export const defaults = {
  inbounds: { listen: "::", listen_port: 443 }, users: { enabled: true },
  nodes: { protocol: "vless", server_port: 443, security: "none" }, reality: { handshake_port: 443 }, services: { enabled: false }, routes: { domain_suffix: [] },
  administrators: { role: "viewer" }
};

export const choices = {
  protocol: { vless: "VLESS", vmess: "VMess", trojan: "Trojan", shadowsocks: "Shadowsocks", anytls: "AnyTLS", hysteria2: "Hysteria2" },
  transport: { tcp: "TCP", ws: "WebSocket", http: "HTTP", http_upgrade: "HTTP Upgrade", grpc: "gRPC" },
  security: { none: "None", tls: "TLS", reality: "REALITY" }
};
