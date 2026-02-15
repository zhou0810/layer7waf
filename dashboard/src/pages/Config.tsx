import { useState, useEffect } from "react";
import { useConfig, useUpdateConfig } from "@/hooks/use-api";
import type { AppConfig } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Select } from "@/components/ui/select";
import { Save, RotateCcw } from "lucide-react";

export function Config() {
  const { data: config, isLoading } = useConfig();
  const updateConfig = useUpdateConfig();

  const [jsonText, setJsonText] = useState("");
  const [draft, setDraft] = useState<AppConfig | null>(null);
  const [saveMsg, setSaveMsg] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    if (config) {
      setDraft(structuredClone(config));
      setJsonText(JSON.stringify(config, null, 2));
    }
  }, [config]);

  function handleSaveStructured() {
    if (!draft) return;
    setSaveMsg(null);
    updateConfig.mutate(draft, {
      onSuccess: () => setSaveMsg({ type: "success", text: "Configuration saved" }),
      onError: (err) => setSaveMsg({ type: "error", text: err.message }),
    });
  }

  function handleSaveJson() {
    setSaveMsg(null);
    try {
      const parsed = JSON.parse(jsonText) as AppConfig;
      updateConfig.mutate(parsed, {
        onSuccess: () => setSaveMsg({ type: "success", text: "Configuration saved" }),
        onError: (err) => setSaveMsg({ type: "error", text: err.message }),
      });
    } catch (e) {
      setSaveMsg({ type: "error", text: `Invalid JSON: ${(e as Error).message}` });
    }
  }

  function handleReset() {
    if (config) {
      setDraft(structuredClone(config));
      setJsonText(JSON.stringify(config, null, 2));
      setSaveMsg(null);
    }
  }

  if (isLoading) {
    return <p className="text-muted-foreground">Loading configuration...</p>;
  }

  if (!draft) {
    return <p className="text-muted-foreground">Failed to load configuration</p>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Configuration</h2>
          <p className="text-muted-foreground text-sm">Edit WAF configuration</p>
        </div>
        <Button variant="outline" size="sm" onClick={handleReset}>
          <RotateCcw className="h-4 w-4 mr-1" />
          Reset
        </Button>
      </div>

      {saveMsg && (
        <div
          className={`rounded-md p-3 text-sm ${
            saveMsg.type === "success"
              ? "bg-emerald-500/20 text-emerald-400"
              : "bg-red-500/20 text-red-400"
          }`}
        >
          {saveMsg.text}
        </div>
      )}

      <Tabs defaultValue="structured">
        <TabsList>
          <TabsTrigger value="structured">Structured Editor</TabsTrigger>
          <TabsTrigger value="json">Raw JSON</TabsTrigger>
        </TabsList>

        <TabsContent value="structured">
          <div className="space-y-4">
            {/* Server */}
            <Card>
              <CardHeader><CardTitle className="text-base">Server</CardTitle></CardHeader>
              <CardContent className="space-y-3">
                <div className="space-y-1">
                  <Label>Listen Addresses</Label>
                  <Input
                    value={draft.server.listen.join(", ")}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        server: {
                          ...draft.server,
                          listen: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                        },
                      })
                    }
                    placeholder="0.0.0.0:8080"
                  />
                  <p className="text-xs text-muted-foreground">Comma-separated</p>
                </div>
                <div className="space-y-1">
                  <Label>Admin Listen</Label>
                  <Input
                    value={draft.server.admin.listen}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        server: {
                          ...draft.server,
                          admin: { ...draft.server.admin, listen: e.target.value },
                        },
                      })
                    }
                  />
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={draft.server.admin.dashboard}
                    onCheckedChange={(v) =>
                      setDraft({
                        ...draft,
                        server: {
                          ...draft.server,
                          admin: { ...draft.server.admin, dashboard: v },
                        },
                      })
                    }
                  />
                  <Label>Dashboard Enabled</Label>
                </div>
              </CardContent>
            </Card>

            {/* Upstreams */}
            <Card>
              <CardHeader><CardTitle className="text-base">Upstreams</CardTitle></CardHeader>
              <CardContent className="space-y-4">
                {draft.upstreams.map((upstream, ui) => (
                  <div key={ui} className="space-y-2 border rounded-md p-3">
                    <div className="space-y-1">
                      <Label>Name</Label>
                      <Input
                        value={upstream.name}
                        onChange={(e) => {
                          const ups = [...draft.upstreams];
                          ups[ui] = { ...upstream, name: e.target.value };
                          setDraft({ ...draft, upstreams: ups });
                        }}
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Servers (addr:weight)</Label>
                      <Input
                        value={upstream.servers.map((s) => `${s.addr}:${s.weight}`).join(", ")}
                        onChange={(e) => {
                          const servers = e.target.value
                            .split(",")
                            .map((s) => s.trim())
                            .filter(Boolean)
                            .map((s) => {
                              const parts = s.split(":");
                              const weight = parts.length > 2 ? parseInt(parts.pop()!, 10) || 1 : 1;
                              return { addr: parts.join(":"), weight };
                            });
                          const ups = [...draft.upstreams];
                          ups[ui] = { ...upstream, servers };
                          setDraft({ ...draft, upstreams: ups });
                        }}
                      />
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>

            {/* WAF */}
            <Card>
              <CardHeader><CardTitle className="text-base">WAF Settings</CardTitle></CardHeader>
              <CardContent className="space-y-3">
                <div className="space-y-1">
                  <Label>Rule Globs</Label>
                  <Input
                    value={draft.waf.rules.join(", ")}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        waf: {
                          ...draft.waf,
                          rules: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                        },
                      })
                    }
                    placeholder="rules/*.conf"
                  />
                </div>
                <div className="space-y-1">
                  <Label>Request Body Limit (bytes)</Label>
                  <Input
                    type="number"
                    value={draft.waf.request_body_limit}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        waf: {
                          ...draft.waf,
                          request_body_limit: parseInt(e.target.value, 10) || 0,
                        },
                      })
                    }
                  />
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={draft.waf.audit_log.enabled}
                    onCheckedChange={(v) =>
                      setDraft({
                        ...draft,
                        waf: {
                          ...draft.waf,
                          audit_log: { ...draft.waf.audit_log, enabled: v },
                        },
                      })
                    }
                  />
                  <Label>Audit Logging</Label>
                </div>
              </CardContent>
            </Card>

            {/* Rate Limit */}
            <Card>
              <CardHeader><CardTitle className="text-base">Rate Limiting</CardTitle></CardHeader>
              <CardContent className="space-y-3">
                <div className="flex items-center gap-2">
                  <Switch
                    checked={draft.rate_limit.enabled}
                    onCheckedChange={(v) =>
                      setDraft({
                        ...draft,
                        rate_limit: { ...draft.rate_limit, enabled: v },
                      })
                    }
                  />
                  <Label>Enabled</Label>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1">
                    <Label>Default RPS</Label>
                    <Input
                      type="number"
                      value={draft.rate_limit.default_rps}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          rate_limit: {
                            ...draft.rate_limit,
                            default_rps: parseInt(e.target.value, 10) || 0,
                          },
                        })
                      }
                    />
                  </div>
                  <div className="space-y-1">
                    <Label>Default Burst</Label>
                    <Input
                      type="number"
                      value={draft.rate_limit.default_burst}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          rate_limit: {
                            ...draft.rate_limit,
                            default_burst: parseInt(e.target.value, 10) || 0,
                          },
                        })
                      }
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* IP Reputation */}
            <Card>
              <CardHeader><CardTitle className="text-base">IP Reputation</CardTitle></CardHeader>
              <CardContent className="space-y-3">
                <div className="space-y-1">
                  <Label>Blocklist Path</Label>
                  <Input
                    value={draft.ip_reputation.blocklist ?? ""}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        ip_reputation: {
                          ...draft.ip_reputation,
                          blocklist: e.target.value || null,
                        },
                      })
                    }
                    placeholder="Not set"
                  />
                </div>
                <div className="space-y-1">
                  <Label>Allowlist Path</Label>
                  <Input
                    value={draft.ip_reputation.allowlist ?? ""}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        ip_reputation: {
                          ...draft.ip_reputation,
                          allowlist: e.target.value || null,
                        },
                      })
                    }
                    placeholder="Not set"
                  />
                </div>
              </CardContent>
            </Card>

            {/* Bot Detection */}
            <Card>
              <CardHeader><CardTitle className="text-base">Bot Detection</CardTitle></CardHeader>
              <CardContent className="space-y-3">
                <div className="flex items-center gap-2">
                  <Switch
                    checked={draft.bot_detection?.enabled ?? false}
                    onCheckedChange={(v) =>
                      setDraft({
                        ...draft,
                        bot_detection: {
                          ...draft.bot_detection,
                          enabled: v,
                          mode: draft.bot_detection?.mode ?? "challenge",
                          js_challenge: draft.bot_detection?.js_challenge ?? {
                            enabled: true,
                            difficulty: 16,
                            ttl_secs: 3600,
                            secret: "",
                          },
                          score_threshold: draft.bot_detection?.score_threshold ?? 0.7,
                          known_bots_allowlist: draft.bot_detection?.known_bots_allowlist ?? [],
                        },
                      })
                    }
                  />
                  <Label>Enabled</Label>
                </div>
                <div className="space-y-1">
                  <Label>Mode</Label>
                  <Select
                    value={draft.bot_detection?.mode ?? "challenge"}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        bot_detection: {
                          ...draft.bot_detection!,
                          mode: e.target.value as "block" | "challenge" | "detect",
                        },
                      })
                    }
                  >
                    <option value="block">Block</option>
                    <option value="challenge">Challenge</option>
                    <option value="detect">Detect</option>
                  </Select>
                </div>
                <div className="space-y-1">
                  <Label>Score Threshold (0.0-1.0)</Label>
                  <Input
                    type="number"
                    step="0.1"
                    min="0"
                    max="1"
                    value={draft.bot_detection?.score_threshold ?? 0.7}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        bot_detection: {
                          ...draft.bot_detection!,
                          score_threshold: parseFloat(e.target.value) || 0.7,
                        },
                      })
                    }
                  />
                </div>
                <div className="space-y-1">
                  <Label>JS Challenge Difficulty (bits)</Label>
                  <Input
                    type="number"
                    value={draft.bot_detection?.js_challenge?.difficulty ?? 16}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        bot_detection: {
                          ...draft.bot_detection!,
                          js_challenge: {
                            ...draft.bot_detection!.js_challenge,
                            difficulty: parseInt(e.target.value, 10) || 16,
                          },
                        },
                      })
                    }
                  />
                </div>
                <div className="space-y-1">
                  <Label>Known Bots Allowlist</Label>
                  <Input
                    value={(draft.bot_detection?.known_bots_allowlist ?? []).join(", ")}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        bot_detection: {
                          ...draft.bot_detection!,
                          known_bots_allowlist: e.target.value
                            .split(",")
                            .map((s) => s.trim())
                            .filter(Boolean),
                        },
                      })
                    }
                    placeholder="Googlebot, Bingbot"
                  />
                  <p className="text-xs text-muted-foreground">Comma-separated bot names to allow</p>
                </div>
              </CardContent>
            </Card>

            <Separator />
            <Button onClick={handleSaveStructured} disabled={updateConfig.isPending}>
              <Save className="h-4 w-4 mr-1" />
              Save Configuration
            </Button>
          </div>
        </TabsContent>

        <TabsContent value="json">
          <div className="space-y-4">
            <Textarea
              value={jsonText}
              onChange={(e) => setJsonText(e.target.value)}
              rows={30}
              className="font-mono text-xs"
            />
            <Button onClick={handleSaveJson} disabled={updateConfig.isPending}>
              <Save className="h-4 w-4 mr-1" />
              Save JSON
            </Button>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
