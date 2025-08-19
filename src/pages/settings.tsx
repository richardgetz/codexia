// app/settings/page.tsx
import { useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Provider, useSettingsStore } from "@/stores/SettingsStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export default function SettingsPage() {
  const {
    providers,
    setProviderApiKey,
    setProviderBaseUrl,
    setProviderModels,
  } = useSettingsStore();
  const [activeSection, setActiveSection] = useState("provider");
  const [selectedProvider, setSelectedProvider] = useState<string>("OpenAI");
  const [newModelName, setNewModelName] = useState("");
  const [editingModelIdx, setEditingModelIdx] = useState<number | null>(null);
  const [editingModelValue, setEditingModelValue] = useState("");
  const [authStatus, setAuthStatus] = useState<
    "idle" | "starting" | "waiting" | "success" | "error"
  >("idle");
  const [authUrl, setAuthUrl] = useState<string>("");
  const [authError, setAuthError] = useState<string>("");
  useEffect(() => {
    const unsubs: Array<() => void> = [];
    (async () => {
      try {
        const un1 = await listen<{ url: string }>("auth-login-url", (e) => {
          if (e.payload?.url) setAuthUrl(e.payload.url);
        });
        const un2 = await listen("auth-login-complete", () => {
          setAuthStatus("success");
        });
        const un3 = await listen<{ error: string }>("auth-login-error", (e) => {
          setAuthStatus("error");
          setAuthError(e.payload?.error || "Login error");
        });
        unsubs.push(() => un1());
        unsubs.push(() => un2());
        unsubs.push(() => un3());
      } catch {}
    })();
    return () => {
      unsubs.forEach((u) => u());
    };
  }, []);
  const providerNames = [
    "OpenAI",
    "Gemini",
    "Ollama",
    "OpenRouter",
  ];

  return (
    <div className="flex h-screen">
      {/* Sidebar */}
      <div className="w-64 border-r bg-muted/30 px-4 space-y-2">
        <Button
          variant={activeSection === "provider" ? "default" : "ghost"}
          className="w-full justify-start"
          onClick={() => setActiveSection("provider")}
        >
          Provider
        </Button>
        <Button
          variant={activeSection === "security" ? "default" : "ghost"}
          className="w-full justify-start"
          onClick={() => setActiveSection("security")}
        >
          Security
        </Button>
        <Button
          variant={activeSection === "working" ? "default" : "ghost"}
          className="w-full justify-start"
          onClick={() => setActiveSection("working")}
        >
          Working Directory
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 px-6 overflow-y-auto">
        {activeSection === "provider" && (
          <div className="grid grid-cols-3 gap-6">
            {/* Left: Providers */}
            <Card className="col-span-1">
              <CardContent className="p-4 space-y-2">
                {providerNames.map((p) => (
                  <Button
                    key={p}
                    variant={selectedProvider === p ? "default" : "ghost"}
                    className="w-full justify-start"
                    onClick={() => setSelectedProvider(p)}
                  >
                    {p}
                  </Button>
                ))}
              </CardContent>
            </Card>

            {/* Right: Models */}
            <Card className="col-span-2">
              <CardContent className="p-4">
                <h2 className="text-lg font-semibold mb-4">
                  {selectedProvider} Models
                </h2>
                <div className="mb-4">
                  <label className="block mb-1 font-medium">API Key</label>
                  <Input
                    type="text"
                    value={providers[selectedProvider as Provider]?.apiKey || ""}
                    onChange={(e) =>
                      setProviderApiKey(selectedProvider as Provider, e.target.value)
                    }
                    placeholder={`Enter API key for ${selectedProvider}`}
                  />
                </div>
                {selectedProvider === "OpenAI" && (
                  <div className="mb-4 space-y-2">
                    <div className="flex items-center gap-2">
                      <Button
                        variant="secondary"
                        onClick={async () => {
                          try {
                            setAuthStatus("starting");
                            setAuthError("");
                            setAuthUrl("");
                            // Start login; URL will be emitted via event
                            await invoke<string>("start_chatgpt_login");
                            setAuthStatus("waiting");
                          } catch (e: any) {
                            setAuthStatus("error");
                            setAuthError(String(e));
                          }
                        }}
                      >
                        Auth with ChatGPT
                      </Button>
                      {authStatus === "waiting" && (
                        <Button
                          variant="destructive"
                          onClick={async () => {
                            try {
                              await invoke("cancel_chatgpt_login");
                              setAuthStatus("idle");
                              setAuthUrl("");
                            } catch (e: any) {
                              setAuthError(String(e));
                            }
                          }}
                        >
                          Cancel
                        </Button>
                      )}
                    </div>
                    {authStatus === "waiting" && (
                      <div className="text-sm text-muted-foreground">
                        Waiting for login to complete...
                      </div>
                    )}
                    {authUrl && (
                      <div className="text-sm">
                        If your browser did not open, open this URL:
                        <div className="mt-1">
                          <a
                            className="text-blue-600 underline break-all"
                            href={authUrl}
                            target="_blank"
                            rel="noreferrer"
                          >
                            {authUrl}
                          </a>
                        </div>
                      </div>
                    )}
                    {authStatus === "success" && (
                      <div className="text-sm text-green-600">
                        Login complete. You can now use OpenAI models.
                      </div>
                    )}
                    {authStatus === "error" && authError && (
                      <div className="text-sm text-red-600">{authError}</div>
                    )}
                  </div>
                )}
                <div className="mb-4">
                  <label className="block mb-1 font-medium">Base URL</label>
                  <Input
                    type="text"
                    value={providers[selectedProvider as Provider]?.baseUrl || ""}
                    onChange={(e) =>
                      setProviderBaseUrl(selectedProvider as Provider, e.target.value)
                    }
                    placeholder={`Enter base URL for ${selectedProvider}`}
                  />
                </div>
                {/* show models */}
                <div className="mb-4">
                  <label className="block mb-1 font-medium">Models</label>
                  <div className="flex gap-2 mb-2">
                    <Input
                      type="text"
                      value={newModelName}
                      onChange={(e) => setNewModelName(e.target.value)}
                      placeholder="Add new model"
                      className="flex-1"
                    />
                    <Button
                      onClick={() => {
                        const trimmed = newModelName.trim();
                        if (!trimmed) return;
                        const provider = selectedProvider as Provider;
                        if ((providers[provider]?.models || []).includes(trimmed)) {
                          setNewModelName("");
                          return;
                        }
                        setProviderModels(provider, [
                          ...(providers[provider]?.models || []),
                          trimmed,
                        ]);
                        setNewModelName("");
                      }}
                    >
                      Add
                    </Button>
                  </div>
                  <ul className="list-disc list-inside mb-2">
                    {(providers[selectedProvider as Provider]?.models || []).map(
                      (model, idx) => (
                        <li
                          key={model + idx}
                          className="flex items-center justify-between py-1"
                        >
                          {editingModelIdx === idx ? (
                            <div className="flex gap-2 flex-1">
                              <Input
                                type="text"
                                value={editingModelValue}
                                onChange={(e) =>
                                  setEditingModelValue(e.target.value)
                                }
                                className="flex-1"
                              />
                              <Button
                                onClick={() => {
                                  const trimmed = editingModelValue.trim();
                                  if (!trimmed) return;
                                  const provider = selectedProvider as Provider;
                                  const newModels = [
                                    ...(providers[provider]?.models || []),
                                  ];
                                  newModels[idx] = trimmed;
                                  setProviderModels(provider, newModels);
                                  setEditingModelIdx(null);
                                  setEditingModelValue("");
                                }}
                              >
                                Save
                              </Button>
                              <Button
                                variant="destructive"
                                onClick={() => {
                                  setEditingModelIdx(null);
                                  setEditingModelValue("");
                                }}
                              >
                                Cancel
                              </Button>
                            </div>
                          ) : (
                            <>
                              <span>{model}</span>
                              <div className="flex gap-2">
                                <Button
                                  size="sm"
                                  onClick={() => {
                                    setEditingModelIdx(idx);
                                    setEditingModelValue(model);
                                  }}
                                >
                                  Edit
                                </Button>
                                <Button
                                  size="sm"
                                  variant="destructive"
                                  onClick={() => {
                                    const provider =
                                      selectedProvider as Provider;
                                    const newModels = (
                                      providers[provider]?.models || []
                                    ).filter((_, i) => i !== idx);
                                    setProviderModels(provider, newModels);
                                  }}
                                >
                                  Delete
                                </Button>
                              </div>
                            </>
                          )}
                        </li>
                      ),
                    )}
                  </ul>
                </div>
              </CardContent>
            </Card>
          </div>
        )}
        {activeSection === "security" && <p>Security Settings</p>}
        {activeSection === "working" && <p>Working Directory Settings</p>}
      </div>
    </div>
  );
}

// Event subscriptions
//
