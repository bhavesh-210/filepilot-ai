import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";
type Mode = "full" | "smart" | "limited";
type Settings = {
  mode: Mode;
  rename: boolean;
  monitoring: boolean;
  personalization: boolean;
  trash_proposals: boolean;
  revision: number;
};
type Folder = {
  id: number;
  path: string;
  active: boolean;
  initial_reviewed: boolean;
};
type Operation = {
  id: number;
  root_id: number;
  source: string;
  destination: string;
  state: string;
  kind: string;
};
type Proposal = {
  id: number;
  source: string;
  destination: string | null;
  category: string;
  reason: string;
  blocked: boolean;
  rename: boolean;
};
type Inventory = {
  root_id: number;
  files: number;
  protected: number;
  errors: string[];
  proposals: Proposal[];
};
type Snapshot = {
  settings: Settings;
  roots: Folder[];
  history: Operation[];
  notice: string;
};
const initial: Snapshot = {
  settings: {
    mode: "full",
    rename: true,
    monitoring: true,
    personalization: true,
    trash_proposals: true,
    revision: 0,
  },
  roots: [],
  history: [],
  notice: "",
};
function App() {
  const [state, setState] = useState(initial),
    [selected, setSelected] = useState<number | null>(null),
    [inventory, setInventory] = useState<Inventory | null>(null),
    [message, setMessage] = useState("Choose a test folder to begin"),
    [busy, setBusy] = useState(false),
    [duplicateGroups, setDuplicateGroups] = useState<string[][]>([]),
    [suggestions, setSuggestions] = useState<
      Record<
        number,
        {
          available: boolean;
          reason?: string;
          model?: string;
          content_used?: boolean;
          ranking?: { category: string; score: number }[];
        }
      >
    >({}),
    [category, setCategory] = useState("Education"),
    [destination, setDestination] = useState("Education");
  const native = "__TAURI_INTERNALS__" in window;
  async function refresh() {
    if (native) setState(await invoke<Snapshot>("snapshot"));
  }
  async function task(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      await refresh();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
    if (!native) return;
    const listening = listen<{ applied: boolean; root_ids?: number[] }>(
      "filepilot-updated",
      (event) => {
        refresh().catch(() => {});
        if (event.payload.applied) {
          setInventory((current) =>
            current &&
            event.payload.root_ids &&
            !event.payload.root_ids.includes(current.root_id)
              ? current
              : null,
          );
          setMessage(
            "Background organization updated files scan for a fresh preview",
          );
        }
      },
    );
    return () => {
      listening.then((stop) => stop());
    };
  }, []);
  const root = state.roots.find((r) => r.id === selected);
  async function scan() {
    if (selected !== null) {
      const value = await invoke<Inventory>("scan", { rootId: selected });
      setInventory(value);
      setMessage(
        value.errors.length
          ? "Scan is incomplete no actions permitted"
          : "Preview ready review the proposed changes",
      );
    }
  }
  async function update(patch: Partial<Settings>) {
    await invoke("update_settings", {
      settings: { ...state.settings, ...patch },
    });
    setInventory(null);
    setMessage("Settings saved pending proposals expired");
  }
  return (
    <div className="app">
      <aside>
        <h1>
          FilePilot <span>AI</span>
        </h1>
        <p>
          Your Files
          <br />
          Your Mac
          <br />
          Your Control
        </p>
        <div className="badge">Local processing</div>
        <h3>Authorized folders</h3>
        {state.roots.map((folder) => (
          <button
            className={selected === folder.id ? "active" : ""}
            key={folder.id}
            disabled={busy}
            onClick={() => {
              setSelected(folder.id);
              setInventory(null);
              setDuplicateGroups([]);
            }}
          >
            {folder.path.split("/").at(-1)}
            {!folder.active ? " access needed" : ""}
          </button>
        ))}
        <button
          disabled={busy || !native}
          onClick={() =>
            task(async () => {
              const id = await invoke<number | null>("choose_folder");
              if (id !== null) {
                setSelected(id);
                setInventory(null);
                setDuplicateGroups([]);
                setMessage(
                  "Folder access granted create category folders then preview",
                );
              }
            })
          }
        >
          Choose folder
        </button>
        <small>
          Closing the window keeps monitoring active
          <br />
          Quit FilePilot to stop the agent
        </small>
      </aside>
      <main>
        <header>
          <div>
            <h2>Your files under control</h2>
            <p>Preview organise and undo on your Mac</p>
          </div>
          <span className="badge">Native alpha</span>
        </header>
        {!native && (
          <div className="notice">
            Browser preview only
            <br />
            Folder access and file operations require the installed macOS app
          </div>
        )}
        <section className="settings">
          <label>
            Autonomy
            <select
              aria-label="Autonomy"
              disabled={busy || !native}
              value={state.settings.mode}
              onChange={(e) =>
                task(() => update({ mode: e.target.value as Mode }))
              }
            >
              <option value="full">Full autonomy</option>
              <option value="smart">Smart approval</option>
              <option value="limited">Limited autonomy</option>
            </select>
          </label>
          {(
            [
              "rename",
              "monitoring",
              "personalization",
              "trash_proposals",
            ] as const
          ).map((key) => (
            <label key={key}>
              <input
                type="checkbox"
                checked={state.settings[key]}
                disabled={busy || !native}
                onChange={(e) =>
                  task(() => update({ [key]: e.target.checked }))
                }
              />
              {key === "rename"
                ? "Normalize spaces in filenames"
                : key === "monitoring"
                  ? "Background monitoring"
                  : key === "trash_proposals"
                    ? "Show separately confirmed Trash actions"
                    : "Learn from explicit corrections"}
            </label>
          ))}
        </section>
        <div role="status" aria-live="polite" className="notice">
          {busy ? "Working locally" : message}
          {state.notice && <p>{state.notice}</p>}
        </div>
        {root && (
          <section>
            <h3>{root.path}</h3>
            <p>
              {root.active
                ? "Folder authorized"
                : "Choose this folder again to renew access"}
            </p>
            <div className="toolbar">
              <button
                disabled={busy || !root.active}
                onClick={() =>
                  task(async () => {
                    await invoke("create_categories", { rootId: selected });
                    setMessage("Category folders created no files moved");
                  })
                }
              >
                Create category folders
              </button>
              <button
                disabled={busy || !root.active}
                onClick={() =>
                  task(async () => {
                    const result = await invoke<{
                      groups: string[][];
                      partial: boolean;
                    }>("duplicates", { rootId: selected });
                    setDuplicateGroups(result.groups);
                    setMessage(
                      result.partial
                        ? "Partial duplicate analysis within safety limits"
                        : "Duplicate analysis complete no deletion authorized",
                    );
                  })
                }
              >
                Find duplicate files
              </button>
              <button
                className="primary"
                disabled={busy || !root.active}
                onClick={() => task(scan)}
              >
                Scan and preview
              </button>
              <button
                disabled={busy || !root.active || !inventory}
                onClick={() =>
                  task(async () => {
                    await invoke("enable_after_preview", { rootId: selected });
                    setMessage(
                      "Initial preview acknowledged current autonomy mode applies to future stable files",
                    );
                  })
                }
              >
                {root.initial_reviewed
                  ? "Initial preview acknowledged"
                  : "Enable after reviewing this preview"}
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  task(async () => {
                    await invoke("revoke", { rootId: selected });
                    setSelected(null);
                    setInventory(null);
                    setDuplicateGroups([]);
                    setMessage("Folder permission revoked in FilePilot");
                  })
                }
              >
                Revoke access
              </button>
            </div>
          </section>
        )}
        {duplicateGroups.length > 0 && (
          <section>
            <h3>Duplicate candidates</h3>
            <p>
              Matching SHA256 contents at inspection time review before taking
              any action
            </p>
            {duplicateGroups.map((group, i) => (
              <div key={i}>
                {group.map((path) => (
                  <p key={path}>
                    {path}{" "}
                    <button
                      disabled={busy || !state.settings.trash_proposals}
                      onClick={() =>
                        task(async () => {
                          const operation = await invoke<number | null>(
                            "trash_file",
                            { rootId: selected, relativePath: path },
                          );
                          if (operation !== null) {
                            setDuplicateGroups([]);
                            setInventory(null);
                            setMessage(
                              "Moved to system Trash after separate confirmation use Undo to restore",
                            );
                          } else {
                            setMessage("Trash cancelled no file moved");
                          }
                        })
                      }
                    >
                      Review Trash action
                    </button>
                  </p>
                ))}
              </div>
            ))}
          </section>
        )}
        {inventory && (
          <section>
            <h3>Organization preview</h3>
            <p>
              {inventory.files} files · {inventory.protected} protected entries
              · {inventory.proposals.length} proposals
            </p>
            {inventory.errors.map((error, i) => (
              <p className="error" key={i}>
                {error}
              </p>
            ))}
            <div className="table">
              <table>
                <thead>
                  <tr>
                    <th>Source</th>
                    <th>Destination and reason</th>
                    <th>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {inventory.proposals.map((p) => (
                    <tr key={p.id}>
                      <td>{p.source}</td>
                      <td>
                        <strong>{p.destination || "Choose a category"}</strong>
                        <small>
                          {p.reason}
                          {p.rename ? " · Includes filename normalization" : ""}
                        </small>
                      </td>
                      <td>
                        <button
                          disabled={busy}
                          onClick={() =>
                            task(async () => {
                              const result = await invoke<{
                                available: boolean;
                                reason?: string;
                                model?: string;
                                content_used?: boolean;
                                ranking?: { category: string; score: number }[];
                              }>("semantic_suggestions", { proposalId: p.id });
                              setSuggestions((current) => ({
                                ...current,
                                [p.id]: result,
                              }));
                              setMessage(
                                result.available
                                  ? "Local semantic suggestions ready explicit review required"
                                  : result.reason || "Local model unavailable",
                              );
                            })
                          }
                        >
                          Inspect locally
                        </button>
                        {suggestions[p.id]?.available && (
                          <div>
                            <small>
                              {suggestions[p.id].model}
                              {suggestions[p.id].content_used
                                ? " · Text inspected locally"
                                : " · Filename only"}
                            </small>
                            {suggestions[p.id].ranking?.map((candidate) => (
                              <button
                                key={candidate.category}
                                disabled={busy}
                                onClick={() =>
                                  task(async () => {
                                    const updated = await invoke<Proposal>(
                                      "review_destination",
                                      {
                                        proposalId: p.id,
                                        category: candidate.category,
                                      },
                                    );
                                    setInventory((current) =>
                                      current
                                        ? {
                                            ...current,
                                            proposals: current.proposals.map(
                                              (row) =>
                                                row.id === p.id ? updated : row,
                                            ),
                                          }
                                        : null,
                                    );
                                    setMessage(
                                      "Destination selected explicitly approve the move separately",
                                    );
                                  })
                                }
                              >
                                {candidate.category}
                              </button>
                            ))}
                          </div>
                        )}
                        <button
                          disabled={busy || p.blocked}
                          onClick={() =>
                            task(async () => {
                              await invoke("apply", { proposalId: p.id });
                              setInventory(null);
                              setMessage(
                                "Operation applied and recorded use Undo in history",
                              );
                            })
                          }
                        >
                          Approve this move
                        </button>
                        <button
                          disabled={
                            busy || p.blocked || !state.settings.personalization
                          }
                          onClick={() =>
                            task(async () => {
                              await invoke("remember_example", {
                                proposalId: p.id,
                              });
                              setInventory(null);
                              setMessage(
                                "Explicit file example saved future nearest matches will still require review",
                              );
                            })
                          }
                        >
                          Remember this destination
                        </button>
                        <button
                          disabled={busy || !state.settings.trash_proposals}
                          onClick={() =>
                            task(async () => {
                              const operation = await invoke<number | null>(
                                "trash_file",
                                { rootId: selected, relativePath: p.source },
                              );
                              if (operation !== null) {
                                setInventory(null);
                                setDuplicateGroups([]);
                                setMessage(
                                  "Moved to system Trash after separate confirmation",
                                );
                              } else {
                                setMessage("Trash cancelled no file moved");
                              }
                            })
                          }
                        >
                          Review Trash action
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}
        {root && (
          <section>
            <h3>Explicit correction or rule</h3>
            <p>Use an existing folder relative to this authorized root</p>
            <div className="toolbar">
              <select
                aria-label="Correction category"
                value={category}
                onChange={(e) => setCategory(e.target.value)}
              >
                {[
                  "Education",
                  "Development",
                  "Professional",
                  "Finance",
                  "Media",
                  "Software",
                  "Personal",
                ].map((c) => (
                  <option key={c}>{c}</option>
                ))}
              </select>
              <input
                aria-label="Destination folder"
                value={destination}
                onChange={(e) => setDestination(e.target.value)}
              />
              <button
                disabled={
                  busy || !root.active || !state.settings.personalization
                }
                onClick={() =>
                  task(async () => {
                    await invoke("correction", {
                      rootId: selected,
                      category,
                      destination,
                      rule: false,
                    });
                    setInventory(null);
                    setMessage("Explicit local correction saved");
                  })
                }
              >
                Save correction
              </button>
              <button
                disabled={busy || !root.active}
                onClick={() =>
                  task(async () => {
                    await invoke("correction", {
                      rootId: selected,
                      category,
                      destination,
                      rule: true,
                    });
                    setInventory(null);
                    setMessage("Limited autonomy rule explicitly authorized");
                  })
                }
              >
                Authorize rule
              </button>
            </div>
          </section>
        )}
        <section>
          <h3>Operation history</h3>
          <button
            disabled={busy || !native}
            onClick={() =>
              task(async () => {
                await invoke("reconcile");
                setInventory(null);
                setMessage(
                  "Interrupted operations reconciled ambiguous cases remain blocked",
                );
              })
            }
          >
            Reconcile interrupted operations
          </button>
          <div className="table">
            <table>
              <thead>
                <tr>
                  <th>Operation</th>
                  <th>Status</th>
                  <th>Recovery</th>
                </tr>
              </thead>
              <tbody>
                {state.history.map((op) => (
                  <tr key={op.id}>
                    <td>
                      {op.kind === "mkdir"
                        ? "Category folder setup"
                        : op.source}
                      <small>
                        to {op.kind === "trash" ? "Trash / " : ""}
                        {op.destination}
                      </small>
                    </td>
                    <td>{op.state}</td>
                    <td>
                      <button
                        disabled={busy || op.state !== "applied"}
                        onClick={() =>
                          task(async () => {
                            await invoke("undo", { operationId: op.id });
                            setInventory(null);
                            setMessage(
                              "Operation undone without overwriting another file",
                            );
                          })
                        }
                      >
                        Undo
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!state.history.length && <p>No file operations yet</p>}
        </section>
        <section>
          <h3>Privacy and startup</h3>
          <div className="toolbar">
            <button
              disabled={busy || !native}
              onClick={() =>
                task(async () => {
                  await invoke("clear_learning");
                  setInventory(null);
                  setMessage("Local correction history cleared");
                })
              }
            >
              Clear learning history
            </button>
            <button
              disabled={busy || !native}
              onClick={() =>
                task(async () => {
                  await invoke("clear_rules");
                  setInventory(null);
                  setMessage("Approved rules cleared");
                })
              }
            >
              Clear authorized rules
            </button>
            <button
              disabled={busy || !native}
              onClick={() =>
                task(async () => {
                  const status = await invoke<{ enabled: boolean }>(
                    "login_item",
                    { enabled: true },
                  );
                  setMessage(
                    status.enabled
                      ? "Startup enabled"
                      : "Check Login Items in System Settings",
                  );
                })
              }
            >
              Enable login item
            </button>
            <button
              disabled={busy || !native}
              onClick={() =>
                task(async () => {
                  await invoke("login_item", { enabled: false });
                  setMessage("Login item disabled");
                })
              }
            >
              Disable login item
            </button>
          </div>
          <p>
            Processing pauses during sleep and resumes with reconciliation
            <br />
            No local processing occurs while your Mac is shut down
          </p>
          <p>
            Trash always opens a separate native confirmation and supports
            guarded Undo
            <br />
            Permanent deletion is unavailable
            <br />
            Local English semantic suggestions use the installed macOS model
            when available and always require explicit review
          </p>
        </section>
      </main>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
