import { useEffect, useState } from "react";
import { FolderOpen, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox, Field, Input, Select } from "@/components/ui/input";
import { Modal } from "@/components/ui/modal";
import {
  forgetHostKey,
  isTauri,
  listNetworkInterfaces,
  setSecret,
} from "@/api/backend";
import { reconnectDefault } from "@/lib/config-defaults";
import { useStore } from "@/state/store";
import type { AuthConfig } from "@/types/generated/AuthConfig";
import type { ForwardKind } from "@/types/generated/ForwardKind";
import type { JumpHost } from "@/types/generated/JumpHost";
import type { NetworkInterface } from "@/types/generated/NetworkInterface";
import type { ProxyConfig } from "@/types/generated/ProxyConfig";
import type { TunnelConfig } from "@/types/generated/TunnelConfig";

interface TunnelFormProps {
  open: boolean;
  initial: TunnelConfig | null;
  onClose: () => void;
}

type AuthMethod = "key" | "password" | "agent" | "keyboard";
type JumpAuthMethod = "key" | "password" | "agent";

interface JumpForm {
  host: string;
  port: string;
  username: string;
  authMethod: JumpAuthMethod;
  keyPath: string;
  passphrase: string;
  password: string;
}

function emptyJump(): JumpForm {
  return {
    host: "",
    port: "22",
    username: "",
    authMethod: "key",
    keyPath: "",
    passphrase: "",
    password: "",
  };
}

interface FormState {
  name: string;
  group: string;
  kind: ForwardKind;
  sshHost: string;
  sshPort: string;
  username: string;
  authMethod: AuthMethod;
  keyPath: string;
  passphrase: string;
  password: string;
  bindAddress: string;
  listenPort: string;
  targetHost: string;
  targetPort: string;
  autoStart: boolean;
  autoReconnect: boolean;
  proxyEnabled: boolean;
  proxyKind: ProxyConfig["kind"];
  proxyHost: string;
  proxyPort: string;
  proxyUser: string;
  proxyPass: string;
  jumps: JumpForm[];
}

function emptyForm(): FormState {
  return {
    name: "",
    group: "",
    kind: "local",
    sshHost: "",
    sshPort: "22",
    username: "",
    authMethod: "key",
    keyPath: "",
    passphrase: "",
    password: "",
    bindAddress: "127.0.0.1",
    listenPort: "",
    targetHost: "",
    targetPort: "",
    autoStart: false,
    autoReconnect: true,
    proxyEnabled: false,
    proxyKind: "http",
    proxyHost: "",
    proxyPort: "1080",
    proxyUser: "",
    proxyPass: "",
    jumps: [],
  };
}

function authMethodOf(kind: AuthConfig["kind"]): AuthMethod {
  switch (kind) {
    case "password":
      return "password";
    case "agent":
      return "agent";
    case "keyboardInteractive":
      return "keyboard";
    default:
      return "key";
  }
}

function fromTunnel(t: TunnelConfig): FormState {
  return {
    name: t.name,
    group: t.group ?? "",
    kind: t.kind,
    sshHost: t.ssh.host,
    sshPort: String(t.ssh.port),
    username: t.ssh.username,
    authMethod: authMethodOf(t.auth.kind),
    keyPath: t.auth.kind === "privateKey" ? t.auth.keyPath : "",
    passphrase: "",
    password: "",
    bindAddress: t.listen.bindAddress,
    listenPort: String(t.listen.port),
    targetHost: t.target?.host ?? "",
    targetPort: t.target ? String(t.target.port) : "",
    autoStart: t.autoStart,
    autoReconnect: t.reconnect.enabled,
    proxyEnabled: t.proxy != null,
    proxyKind: t.proxy?.kind ?? "http",
    proxyHost: t.proxy?.host ?? "",
    proxyPort: t.proxy ? String(t.proxy.port) : "1080",
    proxyUser: "",
    proxyPass: "",
    jumps: (t.jumpHosts ?? []).map((j) => ({
      host: j.endpoint.host,
      port: String(j.endpoint.port),
      username: j.endpoint.username,
      authMethod:
        j.auth.kind === "password"
          ? "password"
          : j.auth.kind === "agent"
            ? "agent"
            : "key",
      keyPath: j.auth.kind === "privateKey" ? j.auth.keyPath : "",
      passphrase: "",
      password: "",
    })),
  };
}

function formatError(e: unknown): string {
  if (e && typeof e === "object") {
    const err = e as { message?: string; fields?: { message: string }[] };
    if (err.fields && err.fields.length > 0) {
      return err.fields.map((f) => f.message).join("; ");
    }
    if (err.message) return err.message;
  }
  return String(e);
}

const KIND_HINT: Record<ForwardKind, string> = {
  local: "Open a local port that reaches a service on the server's side.",
  remote: "Expose a service on this machine via a port on the server.",
  dynamic: "A local SOCKS5 proxy routing anywhere the server can reach.",
};

const FORWARDING: Record<
  ForwardKind,
  { title: string; listenLabel: string; targetLabel: string }
> = {
  local: {
    title: "Forwarding (local → server-side target)",
    listenLabel: "Listen on this machine",
    targetLabel: "Target host (reachable from the server)",
  },
  remote: {
    title: "Reverse forwarding (server → this machine)",
    listenLabel: "Bind on the SSH server",
    targetLabel: "Target host (on this machine)",
  },
  dynamic: {
    title: "SOCKS5 proxy",
    listenLabel: "SOCKS proxy listen address",
    targetLabel: "",
  },
};

export function TunnelForm({ open, initial, onClose }: TunnelFormProps) {
  const saveTunnel = useStore((s) => s.saveTunnel);
  // Fresh state per open: the parent remounts this component with a new key,
  // so props seed the initial state directly (no effect-based syncing).
  const [form, setForm] = useState<FormState>(() =>
    initial ? fromTunnel(initial) : emptyForm(),
  );
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [hostKeyNotice, setHostKeyNotice] = useState<string | null>(null);
  const [interfaces, setInterfaces] = useState<NetworkInterface[]>([]);

  useEffect(() => {
    if (!isTauri) return;
    let active = true;
    void listNetworkInterfaces()
      .then((list) => {
        if (active) setInterfaces(list);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  async function forgetKey() {
    if (!isTauri) return;
    try {
      await forgetHostKey(form.sshHost.trim(), Number(form.sshPort) || 22);
      setHostKeyNotice("Host key forgotten — start the tunnel to re-trust it.");
    } catch (e) {
      setError(formatError(e));
    }
  }

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((prev) => ({ ...prev, [key]: value }));

  async function browseKeyPath() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Select a private key file",
    });
    if (typeof selected === "string") set("keyPath", selected);
  }

  async function buildAuth(id: string): Promise<AuthConfig> {
    if (form.authMethod === "key") {
      let passphraseRef: string | null =
        initial?.auth.kind === "privateKey"
          ? (initial.auth.passphraseRef ?? null)
          : null;
      if (form.passphrase) {
        passphraseRef = `tunnel:${id}:passphrase`;
        if (isTauri) await setSecret(passphraseRef, form.passphrase);
      }
      return {
        kind: "privateKey",
        keyPath: form.keyPath.trim(),
        passphraseRef,
      };
    }
    if (form.authMethod === "agent") {
      return { kind: "agent" };
    }
    const secretRef = `tunnel:${id}:password`;
    if (form.password && isTauri) await setSecret(secretRef, form.password);
    if (form.authMethod === "keyboard") {
      return { kind: "keyboardInteractive", secretRef };
    }
    return { kind: "password", secretRef };
  }

  async function buildProxy(id: string): Promise<ProxyConfig | null> {
    if (!form.proxyEnabled) return null;
    let authRef: string | null = initial?.proxy?.authRef ?? null;
    if (form.proxyUser || form.proxyPass) {
      authRef = `tunnel:${id}:proxy`;
      if (isTauri) {
        await setSecret(authRef, `${form.proxyUser}\n${form.proxyPass}`);
      }
    }
    return {
      kind: form.proxyKind,
      host: form.proxyHost.trim(),
      port: Number(form.proxyPort) || 0,
      authRef,
    };
  }

  async function buildJumps(id: string): Promise<JumpHost[]> {
    const out: JumpHost[] = [];
    for (let i = 0; i < form.jumps.length; i++) {
      const j = form.jumps[i];
      let auth: AuthConfig;
      if (j.authMethod === "agent") {
        auth = { kind: "agent" };
      } else if (j.authMethod === "key") {
        let passphraseRef: string | null = null;
        if (j.passphrase) {
          passphraseRef = `tunnel:${id}:jump${i}:passphrase`;
          if (isTauri) await setSecret(passphraseRef, j.passphrase);
        }
        auth = { kind: "privateKey", keyPath: j.keyPath.trim(), passphraseRef };
      } else {
        const secretRef = `tunnel:${id}:jump${i}:password`;
        if (j.password && isTauri) await setSecret(secretRef, j.password);
        auth = { kind: "password", secretRef };
      }
      out.push({
        endpoint: {
          host: j.host.trim(),
          port: Number(j.port) || 22,
          username: j.username.trim(),
        },
        auth,
      });
    }
    return out;
  }

  const setJump = (index: number, patch: Partial<JumpForm>) =>
    setForm((prev) => ({
      ...prev,
      jumps: prev.jumps.map((j, i) => (i === index ? { ...j, ...patch } : j)),
    }));
  const addJump = () =>
    setForm((prev) => ({ ...prev, jumps: [...prev.jumps, emptyJump()] }));
  const removeJump = (index: number) =>
    setForm((prev) => ({
      ...prev,
      jumps: prev.jumps.filter((_, i) => i !== index),
    }));

  async function submit() {
    setError(null);
    setSaving(true);
    try {
      const id = initial?.id ?? crypto.randomUUID();
      const now = new Date().toISOString();
      const auth = await buildAuth(id);
      const proxy = await buildProxy(id);
      const jumpHosts = await buildJumps(id);
      const tunnel: TunnelConfig = {
        id,
        name: form.name.trim(),
        description: null,
        kind: form.kind,
        enabled: initial?.enabled ?? true,
        autoStart: form.autoStart,
        reconnect: { ...reconnectDefault(), enabled: form.autoReconnect },
        ssh: {
          host: form.sshHost.trim(),
          port: Number(form.sshPort) || 0,
          username: form.username.trim(),
        },
        auth,
        listen: {
          bindAddress: form.bindAddress.trim(),
          port: Number(form.listenPort) || 0,
        },
        target:
          form.kind === "dynamic"
            ? null
            : {
                host: form.targetHost.trim(),
                port: Number(form.targetPort) || 0,
              },
        proxy,
        jumpHosts,
        keepAliveSeconds: null,
        connectTimeoutMs: null,
        compression: null,
        group: form.group.trim() || null,
        tags: initial?.tags ?? [],
        createdAt: initial?.createdAt ?? now,
        updatedAt: now,
      };
      await saveTunnel(tunnel);
      onClose();
    } catch (e) {
      setError(formatError(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title={initial ? "Edit tunnel" : "New tunnel"}
      onClose={onClose}
      footer={
        <>
          <Button variant="outline" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button variant="brand" onClick={submit} disabled={saving}>
            {saving ? "Saving…" : "Save tunnel"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {error && (
          <div className="border-destructive/40 bg-destructive/10 text-destructive rounded-md border px-3 py-2 text-xs">
            {error}
          </div>
        )}

        <div className="grid grid-cols-2 gap-3">
          <Field label="Name">
            <Input
              value={form.name}
              onChange={(e) => set("name", e.currentTarget.value)}
              placeholder="Prod Postgres"
            />
          </Field>
          <Field label="Group" hint="Optional">
            <Input
              value={form.group}
              onChange={(e) => set("group", e.currentTarget.value)}
              placeholder="Production"
            />
          </Field>
        </div>

        <Field label="Tunnel type" hint={KIND_HINT[form.kind]}>
          <Select
            value={form.kind}
            onChange={(e) => set("kind", e.currentTarget.value as ForwardKind)}
          >
            <option value="local">Local (-L) — reach a remote service</option>
            <option value="remote">Remote (-R) — expose a local service</option>
            <option value="dynamic">Dynamic (-D) — SOCKS5 proxy</option>
          </Select>
        </Field>

        <Section title="SSH server (the machine you tunnel through)">
          <div className="grid grid-cols-[1fr_88px] gap-3">
            <Field label="Host">
              <Input
                value={form.sshHost}
                onChange={(e) => set("sshHost", e.currentTarget.value)}
                placeholder="bastion.example.com"
              />
            </Field>
            <Field label="Port">
              <Input
                value={form.sshPort}
                onChange={(e) => set("sshPort", e.currentTarget.value)}
                inputMode="numeric"
              />
            </Field>
          </div>
          <Field label="Username">
            <Input
              value={form.username}
              onChange={(e) => set("username", e.currentTarget.value)}
              placeholder="sam"
            />
          </Field>
          {isTauri && (
            <div className="flex flex-wrap items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void forgetKey()}
              >
                Forget saved host key
              </Button>
              {hostKeyNotice && (
                <span className="text-success text-xs">{hostKeyNotice}</span>
              )}
            </div>
          )}
        </Section>

        <Section title="Authentication">
          <Field label="Method">
            <Select
              value={form.authMethod}
              onChange={(e) =>
                set("authMethod", e.currentTarget.value as AuthMethod)
              }
            >
              <option value="key">Private key</option>
              <option value="password">Password</option>
              <option value="agent">SSH agent</option>
              <option value="keyboard">Keyboard-interactive (2FA)</option>
            </Select>
          </Field>
          {form.authMethod === "key" && (
            <div className="grid grid-cols-2 gap-3">
              <Field label="Private key path">
                <div className="flex gap-2">
                  <Input
                    className="flex-1"
                    value={form.keyPath}
                    onChange={(e) => set("keyPath", e.currentTarget.value)}
                    placeholder="~/.ssh/id_ed25519"
                  />
                  {isTauri && (
                    <Button
                      variant="outline"
                      size="icon"
                      title="Browse…"
                      onClick={() => void browseKeyPath()}
                    >
                      <FolderOpen />
                    </Button>
                  )}
                </div>
              </Field>
              <Field
                label="Passphrase"
                hint={
                  initial
                    ? "Leave blank to keep existing"
                    : "If the key is encrypted"
                }
              >
                <Input
                  type="password"
                  value={form.passphrase}
                  onChange={(e) => set("passphrase", e.currentTarget.value)}
                />
              </Field>
            </div>
          )}
          {(form.authMethod === "password" ||
            form.authMethod === "keyboard") && (
            <Field
              label={
                form.authMethod === "keyboard"
                  ? "Response / password"
                  : "Password"
              }
              hint={
                initial
                  ? "Leave blank to keep existing"
                  : form.authMethod === "keyboard"
                    ? "Used to answer the server's prompts (not for live OTP codes)"
                    : "Stored in the OS keychain"
              }
            >
              <Input
                type="password"
                value={form.password}
                onChange={(e) => set("password", e.currentTarget.value)}
              />
            </Field>
          )}
          {form.authMethod === "agent" && (
            <p className="text-muted-foreground text-xs">
              Authenticates using the keys held by your running SSH agent — no
              secret is stored.
            </p>
          )}
        </Section>

        <Section title={FORWARDING[form.kind].title}>
          <div className="grid grid-cols-[1fr_88px] gap-3">
            <Field
              label={FORWARDING[form.kind].listenLabel}
              hint="127.0.0.1 = only this machine"
            >
              <Select
                value={form.bindAddress}
                onChange={(e) => set("bindAddress", e.currentTarget.value)}
              >
                <option value="127.0.0.1">127.0.0.1 (localhost)</option>
                <option value="0.0.0.0">0.0.0.0 (everyone)</option>
                {interfaces
                  .filter(
                    (i) => i.address !== "127.0.0.1" && i.address !== "0.0.0.0",
                  )
                  .map((i) => (
                    <option key={i.address} value={i.address}>
                      {i.address} ({i.name})
                    </option>
                  ))}
                {form.bindAddress &&
                  form.bindAddress !== "127.0.0.1" &&
                  form.bindAddress !== "0.0.0.0" &&
                  !interfaces.some((i) => i.address === form.bindAddress) && (
                    <option value={form.bindAddress}>{form.bindAddress}</option>
                  )}
              </Select>
            </Field>
            <Field label="Port">
              <Input
                value={form.listenPort}
                onChange={(e) => set("listenPort", e.currentTarget.value)}
                inputMode="numeric"
                placeholder="5432"
              />
            </Field>
          </div>
          {form.kind !== "dynamic" && (
            <div className="grid grid-cols-[1fr_88px] gap-3">
              <Field label={FORWARDING[form.kind].targetLabel}>
                <Input
                  value={form.targetHost}
                  onChange={(e) => set("targetHost", e.currentTarget.value)}
                  placeholder="db.internal"
                />
              </Field>
              <Field label="Port">
                <Input
                  value={form.targetPort}
                  onChange={(e) => set("targetPort", e.currentTarget.value)}
                  inputMode="numeric"
                  placeholder="5432"
                />
              </Field>
            </div>
          )}
        </Section>

        <Section title="Proxy (optional — reach the SSH server through a proxy)">
          <Checkbox
            checked={form.proxyEnabled}
            onChange={(v) => set("proxyEnabled", v)}
            label="Connect through a proxy"
          />
          {form.proxyEnabled && (
            <>
              <div className="grid grid-cols-[110px_1fr_78px] gap-3">
                <Field label="Type">
                  <Select
                    value={form.proxyKind}
                    onChange={(e) =>
                      set(
                        "proxyKind",
                        e.currentTarget.value as ProxyConfig["kind"],
                      )
                    }
                  >
                    <option value="http">HTTP</option>
                    <option value="socks5">SOCKS5</option>
                  </Select>
                </Field>
                <Field label="Proxy host">
                  <Input
                    value={form.proxyHost}
                    onChange={(e) => set("proxyHost", e.currentTarget.value)}
                    placeholder="proxy.example.com"
                  />
                </Field>
                <Field label="Port">
                  <Input
                    value={form.proxyPort}
                    onChange={(e) => set("proxyPort", e.currentTarget.value)}
                    inputMode="numeric"
                  />
                </Field>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <Field label="Username" hint="Optional">
                  <Input
                    value={form.proxyUser}
                    onChange={(e) => set("proxyUser", e.currentTarget.value)}
                  />
                </Field>
                <Field
                  label="Password"
                  hint={initial ? "Leave blank to keep existing" : "Optional"}
                >
                  <Input
                    type="password"
                    value={form.proxyPass}
                    onChange={(e) => set("proxyPass", e.currentTarget.value)}
                  />
                </Field>
              </div>
            </>
          )}
        </Section>

        <Section title="Jump hosts (optional — hop through bastions, in order)">
          {form.jumps.map((jump, i) => (
            <div key={i} className="space-y-2 rounded-md border p-2.5">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground text-xs font-medium">
                  Hop {i + 1}
                </span>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  title="Remove"
                  onClick={() => removeJump(i)}
                >
                  <Trash2 />
                </Button>
              </div>
              <div className="grid grid-cols-[1fr_70px] gap-2">
                <Input
                  value={jump.host}
                  onChange={(e) => setJump(i, { host: e.currentTarget.value })}
                  placeholder="bastion.example.com"
                />
                <Input
                  value={jump.port}
                  onChange={(e) => setJump(i, { port: e.currentTarget.value })}
                  inputMode="numeric"
                />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <Input
                  value={jump.username}
                  onChange={(e) =>
                    setJump(i, { username: e.currentTarget.value })
                  }
                  placeholder="user"
                />
                <Select
                  value={jump.authMethod}
                  onChange={(e) =>
                    setJump(i, {
                      authMethod: e.currentTarget.value as JumpAuthMethod,
                    })
                  }
                >
                  <option value="key">Private key</option>
                  <option value="password">Password</option>
                  <option value="agent">SSH agent</option>
                </Select>
              </div>
              {jump.authMethod === "key" && (
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    value={jump.keyPath}
                    onChange={(e) =>
                      setJump(i, { keyPath: e.currentTarget.value })
                    }
                    placeholder="~/.ssh/id_ed25519"
                  />
                  <Input
                    type="password"
                    value={jump.passphrase}
                    onChange={(e) =>
                      setJump(i, { passphrase: e.currentTarget.value })
                    }
                    placeholder="passphrase"
                  />
                </div>
              )}
              {jump.authMethod === "password" && (
                <Input
                  type="password"
                  value={jump.password}
                  onChange={(e) =>
                    setJump(i, { password: e.currentTarget.value })
                  }
                  placeholder="password"
                />
              )}
            </div>
          ))}
          <Button variant="outline" size="sm" onClick={addJump}>
            <Plus />
            Add jump host
          </Button>
        </Section>

        <div className="flex flex-col gap-2">
          <Checkbox
            checked={form.autoStart}
            onChange={(v) => set("autoStart", v)}
            label="Auto-start when the app launches"
          />
          <Checkbox
            checked={form.autoReconnect}
            onChange={(v) => set("autoReconnect", v)}
            label="Auto-reconnect if the tunnel drops"
          />
        </div>
      </div>
    </Modal>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-3 rounded-lg border p-3">
      <p className="text-muted-foreground text-xs font-semibold">{title}</p>
      {children}
    </div>
  );
}
