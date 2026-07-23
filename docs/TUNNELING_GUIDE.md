# The Plain-English Guide to SSH Tunneling

No jargon. This guide explains **what an SSH tunnel is**, **the three kinds of
forwarding**, and **every option in Tunnelium** — each with a real-life scenario so
you know *when* you'd actually use it.

> **Skip ahead:**
> [What is a tunnel?](#1-what-is-an-ssh-tunnel-the-30-second-version) ·
> [Local](#3-local-port-forwarding-pull-something-to-me) ·
> [Remote](#4-remote-port-forwarding-push-something-out-to-them) ·
> [Dynamic](#5-dynamic-port-forwarding-a-personal-proxyvpn-lite) ·
> [Every option explained](#6-every-option-explained) ·
> [Recipes](#8-common-recipes)

---

## 1. What is an SSH tunnel? (the 30-second version)

Imagine two buildings connected by a **private, locked, armored hallway**. Anyone in
one building can walk to the other without stepping outside — safe from the weather and
from prying eyes.

- The two "buildings" are **your computer** and a **remote server** you can log into with SSH.
- The "armored hallway" is the **encrypted SSH connection** between them.
- **Tunneling** means: send some other app's network traffic *through that hallway*
  instead of over the open internet.

Why do that?
- **Reach things you normally can't** — a database or admin page that only listens
  "inside" the server.
- **Protect traffic** on untrusted networks (coffee-shop Wi-Fi) by wrapping it in SSH's encryption.
- **Appear to come from the server** instead of from your own location.

You need one thing to start: **an SSH login** to a server (a host, a username, and either
a password or a key). That's it.

---

## 2. A tiny bit of vocabulary

- **Port** — a numbered "door" on a computer that a specific service listens behind.
  Web = 80/443, Postgres = 5432, Redis = 6379, RDP = 3389. Think apartment numbers at one street address (the IP).
- **localhost / 127.0.0.1** — "this computer, talking to itself." Services bound to
  localhost are only reachable *from that same machine* — which is exactly why you often need a tunnel.
- **SSH server** — the machine you tunnel *through*. Also called the "jump" or "gateway".
- **Forwarding** — pushing traffic from one port through the tunnel to another port. The
  three flavors below differ only in **which direction** traffic flows and **who opens the door**.

---

## 3. Local port forwarding ("pull something *to* me")

**In one sentence:** open a door **on your computer** that secretly leads to a service
**over on the server's side**.

You connect to `localhost:<some port>` on your own machine, and it comes out on the
*other* end of the tunnel, as if the server itself made the request.

```
Your app ──► localhost:5432 (on your PC) ══SSH tunnel══► server ──► database:5432
```

### When you'd use it (scenarios)

- **The database only listens on the server.** Your company's Postgres is bound to
  `localhost` on a server for safety — no outside connections allowed. Make a **local**
  tunnel: your `localhost:5432` → the server's `localhost:5432`. Now your database tool
  connects to `localhost:5432` and it "just works," fully encrypted.
- **An internal admin page.** There's a dashboard at `http://10.0.0.5:8080` that only
  exists inside the office network. Tunnel your `localhost:8080` → `10.0.0.5:8080` and
  open it in your browser from home.
- **Reach a device behind the server.** A printer, a router page, an IoT box on the
  remote LAN — point the tunnel's target at that device's IP and port.

### What you fill in

- **Listen port** — the door number on *your* PC (e.g. `5432`).
- **Target host + port** — what to reach, *from the server's point of view* (e.g.
  `localhost:5432`, or `10.0.0.5:8080`).

> Rule of thumb: **Local = "I want to get *in* to something on the other side."**

---

## 4. Remote port forwarding ("push something *out* to them")

**In one sentence:** open a door **on the server** that secretly leads back to a service
running **on your computer**.

It's local forwarding in reverse. People (or programs) on the server side connect to a
port *on the server*, and the traffic pops out on *your* machine.

```
Someone ──► server:9000 ══SSH tunnel══► your PC ──► localhost:3000 (your app)
```

### When you'd use it (scenarios)

- **Show your work-in-progress to a colleague.** Your website runs on your laptop at
  `localhost:3000`. Make a **remote** tunnel so `server:9000` maps back to your
  `localhost:3000`. Anyone who can reach the server can now see your local site — no
  deploy needed.
- **Let a webhook reach your laptop.** A payment provider or CI system needs to POST to a
  public URL, but your dev app is on your laptop behind a router. Expose it via the server.
- **Temporary access to a machine that can't accept inbound connections.** Your device
  can *make* an outbound SSH connection but nothing can dial *in* to it (typical for
  laptops/home networks). Remote forwarding flips the direction so it can still offer a service.

### What you fill in

- **Listen (bind) on the server** — the door number *on the server* (e.g. `9000`).
- **Target host + port** — what to reach *from your machine's* point of view (e.g.
  `localhost:3000`).

> Rule of thumb: **Remote = "I want to expose something *out* to the other side."**
> ⚠️ If you want *other people* (not just the server itself) to reach it, the server's
> SSH config must allow non-local binds (`GatewayPorts`). See [Gotchas](#7-gotchas--safety).

---

## 5. Dynamic port forwarding ("a personal proxy / VPN-lite")

**In one sentence:** turn the SSH connection into a **general-purpose proxy** — one door
on your PC that can reach *anything the server can reach*, decided per request.

With local/remote forwarding you pick **one** fixed destination. Dynamic forwarding
doesn't — your app says "I want site X" each time, and the tunnel goes and fetches it
from the server's side. This is a **SOCKS5 proxy**.

```
Browser (SOCKS → localhost:1080) ══SSH tunnel══► server ──► anywhere the server can go
```

### When you'd use it (scenarios)

- **Browse safely on public Wi-Fi.** Point your browser's SOCKS proxy at
  `localhost:1080`. Every page loads *through* your trusted server, encrypted across the
  coffee shop's network.
- **Appear to be "at the office" (or in another country).** Websites and internal tools
  see the **server's** location/IP, not yours. Great for reaching geo- or IP-restricted internal apps.
- **Reach a whole internal network, not just one box.** Instead of one tunnel per
  internal service, one dynamic tunnel lets your browser (or any SOCKS-aware app) reach
  *all* of them.

### What you fill in

- Just a **listen port** for the SOCKS proxy (e.g. `1080`). No fixed target — that's the point.
- Then set your browser/app to use **SOCKS5 → `localhost:1080`**.

> Rule of thumb: **Dynamic = "Give me a proxy; I'll decide where to go each time."**

---

## Quick comparison

| | Direction | You choose the target… | Classic use |
| --- | --- | --- | --- |
| **Local** | Pull *in* to you | once, up front | Reach a remote DB / internal site |
| **Remote** | Push *out* to them | once, up front | Expose your local app to others |
| **Dynamic** | Proxy for you | per request (SOCKS) | Secure browsing / reach many services |

---

## 6. Every option explained

These are the fields and toggles Tunnelium gives you when creating or editing a tunnel.

### Connection

- **SSH host / port / username** — *the server you tunnel through*, and how you log in.
  Port is almost always `22`.
- **Authentication method** — *how you prove who you are:*
  - **Password** — the account password. Simplest; least safe if reused.
  - **Private key (+ passphrase)** — a key file (like a very long, unguessable password)
    that pairs with a "public key" the server already trusts. The **passphrase** unlocks
    the key on your side. Recommended.
  - **SSH agent** — a helper already holding your unlocked keys, so you don't re-type the
    passphrase. Convenient on machines you use daily.
  - **Keyboard-interactive** — the server asks question(s), often for **2-factor / OTP** codes.

### What gets forwarded

- **Tunnel type** — Local / Remote / Dynamic (explained above).
- **Listen (bind) address** — *which "front door" the listener uses:*
  - `127.0.0.1` (default, safest) — only **this** computer can use the tunnel.
  - `0.0.0.0` — **every** device on your network can use it. Powerful but exposes the
    forwarded service to your whole LAN. Tunnelium warns you when you pick this.
  - **A specific adapter/IP** (e.g. your Ethernet or VPN interface) — only that network
    can use it. Useful on multi-homed machines.
- **Listen port** — the door number for the listener.
- **Target host / port** — the final destination (not shown for Dynamic — the app decides per request).

### Getting to the server the hard way

- **Proxy** — *if you can't reach the SSH server directly.* Behind a corporate HTTP proxy
  or a SOCKS gateway? Enter it here and Tunnelium dials the SSH server *through* the proxy.
- **Jump host(s) / bastion** — *hop through a gateway first.* Many networks put one locked-down
  "bastion" server at the edge; you must go **through** it to reach anything inside. Add the
  bastion as a jump host and Tunnelium chains the connection for you (like SSH's `ProxyJump`).

### Reliability & automation

- **Keepalive interval** — sends a tiny "still here?" ping so idle tunnels aren't silently
  dropped by firewalls/routers.
- **Connection timeout** — how long to wait before giving up on a stalled connect.
- **Compression** — squeezes data before sending; can help on *slow* links, usually not worth it on fast ones.
- **Auto-start** — start this tunnel automatically when Tunnelium launches.
- **Auto-reconnect** — if the tunnel drops (laptop sleep, Wi-Fi hiccup), keep retrying —
  waiting a little longer between each try (**backoff**) so you don't hammer the server.
- **Enabled / disabled** — keep a tunnel's settings around without it counting toward
  "Start all" or auto-start.

### Security

- **Host-key verification** — the first time you connect, Tunnelium remembers the
  server's unique **fingerprint**. If it ever changes, you're warned (it could mean the
  server was replaced — or something malicious). Leave this **on**.
- **Secrets storage** — passwords and passphrases are kept in your **operating system's
  keychain**, never written in plain text and never included when you export your config.

### Organization & visibility

- **Name / description / group / tags** — label and file your tunnels so a long list stays sane.
- **Live log** — a running account of connects, drops, retries, and errors for that tunnel.
- **Stats** — data sent/received, uptime, and how many connections are active right now.

### App-wide

- **Start all / Stop all** — flip every enabled tunnel at once.
- **Start on system boot** — launch Tunnelium automatically when you log in to your computer.
- **Minimize to tray / notifications / theme** — quality-of-life: run quietly in the
  background, get told when something connects or fails, and pick light/dark.

---

## 7. Gotchas & safety

- **`0.0.0.0` exposes the service to your whole network.** Only use it when you *mean* to
  share; otherwise keep `127.0.0.1`.
- **Remote forwarding to *other people* needs the server's permission.** By default a
  remote-forwarded port is reachable only *from the server itself*. To let others reach
  it, the server's SSH config must set `GatewayPorts clientspecified` (or `yes`) — that's a
  server-admin setting, not something the client can force.
- **Only tunnel through servers you trust and are allowed to use.** A dynamic/SOCKS tunnel
  routes your traffic through the server operator; local/remote tunnels can cross network
  boundaries your organization may have policies about. Use this on your own or authorized systems.
- **Keep host-key verification on.** A silent key change is exactly what a
  man-in-the-middle attack looks like.
- **"Address already in use"** means the listen port is taken — pick another, or stop
  whatever's using it.

---

## 8. Common recipes

> Fill in your own server/host/port; these show the *shape* of each setup.

**Reach a remote Postgres that only listens on the server**
- Type: **Local** · Listen: `127.0.0.1:5432` · Target: `localhost:5432` · SSH: your server.
- Then connect your DB tool to `localhost:5432`.

**Open a remote web dashboard that's only on the internal LAN**
- Type: **Local** · Listen: `127.0.0.1:8080` · Target: `10.0.0.5:8080` · SSH: your gateway.
- Then browse `http://localhost:8080`.

**Show your laptop's dev site to a teammate via the server**
- Type: **Remote** · Bind on server: `0.0.0.0:9000` · Target: `localhost:3000`.
- (Server needs `GatewayPorts` for non-local access.) Teammate opens `http://server:9000`.

**Secure browsing on public Wi-Fi**
- Type: **Dynamic** · Listen: `127.0.0.1:1080`.
- Set your browser's proxy to **SOCKS5 `localhost:1080`**.

**Reach a database behind a bastion**
- Type: **Local** · Target: `db.internal:5432` · **Jump host:** the bastion · SSH: the
  inner host (or the bastion, targeting the DB).

---

## 9. Still confused? The one thing to remember

- Want to **get to** something on the far side → **Local**.
- Want to **hand out** something from your side → **Remote**.
- Want a **general proxy** that goes wherever you ask → **Dynamic**.

Everything else — keys, proxies, jump hosts, auto-reconnect — is just making that one
tunnel more convenient, more reliable, or able to reach harder-to-get-to places.
