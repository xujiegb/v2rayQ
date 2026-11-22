# Desktop and router network services

The client controls a fixed pair of operating-system services. Install the engine, service definitions and proxy profiles as an administrator. Keep any files consumed by the privileged services writable only by administrators. Each service should shut the engine down cleanly so the engine can restore routes and firewall state.

| Host | TUN service | Transparent proxy service |
| --- | --- | --- |
| Linux with systemd | `v2rayq-tun.service` | `v2rayq-transparent.service` |
| OpenWrt with procd | `/etc/init.d/v2rayq-tun` | `/etc/init.d/v2rayq-transparent` |
| macOS launchd | `system/org.v2rayq.tun` | `system/org.v2rayq.transparent` |
| FreeBSD rc | `/usr/local/etc/rc.d/v2rayq_tun` | `/usr/local/etc/rc.d/v2rayq_transparent` |
| Windows SCM | `v2rayq-tun` | `v2rayq-transparent` |

Install the custom Core and service definitions once its launcher and configuration format are available. The Linux service units should declare a conflict if the two modes manage the same routes or firewall rules. On OpenWrt, provide procd `running`, `start` and `stop` support. On macOS, bootstrap the jobs into the system domain and leave `KeepAlive` off so `launchctl kill SIGTERM` can stop them. On FreeBSD, provide `onestart`, `onestop` and `onestatus` via rc.subr. Windows needs a service compatible with the Service Control Manager and an account authorized to control it.

```sh
v2rayq-cli status
v2rayq-cli tun on
v2rayq-cli tun off
v2rayq-cli transparent on
v2rayq-cli transparent off
```

On macOS, Linux and FreeBSD, CLI actions use `sudo -k` to request authentication according to the host's sudo policy. egui and local Web launch `sudo -A -k`. Set `SUDO_ASKPASS` to a trusted, installed graphical askpass program or configure `Path askpass` in `sudo.conf`. Password entry stays with sudo and the configured program. FreeBSD installations may need to install `sudo` separately. Cockpit uses its host superuser session, and LuCI uses the dedicated service ACL. Neither panel needs a sudo password field.
