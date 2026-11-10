import { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Setup } from "./components/Setup";
import { Preferences } from "./components/Preferences";
import { Collections } from "./components/Collections";
import { Review } from "./components/Review";
import {
  Inventory,
  Preview,
  Settings,
  Snapshot,
  categories,
  folderName,
} from "./types";
import "./style.css";
const initial: Snapshot = {
  settings: {
    mode: "full",
    rename: true,
    monitoring: true,
    personalization: true,
    trash_proposals: true,
    revision: 0,
  },
  preferences: { reuse_existing: true },
  roots: [],
  inventories: [],
  history: [],
  notice: "",
};
type Page =
  | "overview"
  | "review"
  | "collections"
  | "folders"
  | "activity"
  | "preferences";
const pages: { id: Page; label: string; symbol: string }[] = [
  { id: "overview", label: "Overview", symbol: "◫" },
  { id: "review", label: "Review inbox", symbol: "▤" },
  { id: "collections", label: "Collections", symbol: "▱" },
  { id: "folders", label: "Your folders", symbol: "▱" },
  { id: "activity", label: "Activity", symbol: "↶" },
  { id: "preferences", label: "Preferences", symbol: "☷" },
];
function App() {
  const native = "__TAURI_INTERNALS__" in window;
  const refreshSequence = useRef(0);
  const [startup, setStartup] = useState(true);
  const [state, setState] = useState(initial),
    [loaded, setLoaded] = useState(false),
    [page, setPage] = useState<Page>("overview"),
    [busy, setBusy] = useState(false),
    [message, setMessage] = useState(""),
    [selected, setSelected] = useState<number | null>(null),
    [previews, setPreviews] = useState<Preview[]>([]),
    [duplicateGroups, setDuplicateGroups] = useState<string[][]>([]),
    [category, setCategory] = useState("Education"),
    [destination, setDestination] = useState("Education");
  const refresh = useCallback(async () => {
    const sequence = ++refreshSequence.current;
    if (native) {
      const next = await invoke<Snapshot>("snapshot");
      if (sequence !== refreshSequence.current) return;
      setState({
        ...next,
        preferences: next.preferences || initial.preferences,
        inventories: next.inventories || [],
      });
    }
    setLoaded(true);
  }, [native]);
  async function task(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      await refresh();
    } catch (error) {
      setMessage(
        String(error) +
          " Check Activity for any completed or interrupted changes",
      );
      try {
        await refresh();
      } catch {
        /* Preserve the original action error */
      }
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    let disposed = false;
    refresh().catch((e) => {
      if (!disposed) setMessage(String(e));
    });
    if (!native) return;
    const subscription = listen<{ applied: boolean; root_ids?: number[] }>(
      "filepilot-updated",
      () => {
        if (!disposed) refresh().catch((e) => setMessage(String(e)));
      },
    );
    return () => {
      disposed = true;
      subscription.then((stop) => stop()).catch(() => {});
    };
  }, [native, refresh]);
  const active = state.roots.filter((r) => r.active);
  const pendingSetup = active.filter((r) => !r.initial_reviewed);
  const needsSetup = active.length === 0 || pendingSetup.length > 0;
  const root =
    state.roots.find((r) => r.id === selected) || active[0] || state.roots[0];
  const inventory = state.inventories.find((i) => i.root_id === root?.id);
  const pending = state.inventories.reduce((n, i) => n + i.proposals.length, 0);
  const applied = state.history.filter(
    (op) => op.state === "applied" && op.kind !== "mkdir",
  ).length;
  const protectedCount = state.inventories.reduce((n, i) => n + i.protected, 0);
  async function update(patch: Partial<Settings>) {
    await invoke("update_settings", {
      settings: { ...state.settings, ...patch },
    });
    setPreviews([]);
    setMessage("Preferences saved pending proposals will be refreshed");
  }
  async function structure(reuse: boolean) {
    await invoke("update_preferences", {
      preferences: { reuse_existing: reuse },
    });
    setPreviews([]);
    setMessage("Folder preferences saved");
  }
  async function choose() {
    const result = await invoke<{
      ids: number[];
      errors: string[];
      cancelled: boolean;
    }>("choose_folders");
    setPreviews([]);
    setDuplicateGroups([]);
    if (result.ids[0]) setSelected(result.ids[0]);
    setMessage(
      result.errors.length
        ? result.errors.join("\n")
        : result.cancelled
          ? "Folder selection cancelled"
          : "Access saved for the folders you chose",
    );
  }
  async function prepare() {
    setPreviews([]);
    const results: Preview[] = [];
    for (const folder of pendingSetup) {
      results.push(
        await invoke<Preview>("prepare_setup", { rootId: folder.id }),
      );
    }
    setPreviews(results);
    setMessage("First cleanup preview ready no files changed");
  }
  async function start() {
    try {
      for (const preview of previews.filter((p) =>
        pendingSetup.some((r) => r.id === p.inventory.root_id),
      ))
        await invoke("finish_setup", {
          rootId: preview.inventory.root_id,
          token: preview.token,
        });
      let startupNote = "";
      if (startup) {
        try {
          const status = await invoke<{ enabled: boolean }>("login_item", {
            enabled: true,
          });
          if (!status.enabled)
            startupNote =
              " Check Login Items in System Settings to finish startup access";
        } catch {
          startupNote =
            " Maintenance is ready enable startup later in Preferences";
        }
      }
      setPreviews([]);
      setPage("overview");
      setMessage(
        "Setup complete FilePilot will maintain your approved folders under your chosen mode" +
          startupNote,
      );
    } finally {
      await refresh();
    }
  }
  async function rescan() {
    if (root) {
      const next = await invoke<Inventory>("scan", { rootId: root.id });
      setState((s) => ({
        ...s,
        inventories: [
          ...s.inventories.filter((i) => i.root_id !== root.id),
          next,
        ],
      }));
    }
  }
  const modeName = {
    full: "Full autonomy",
    smart: "Smart approval",
    limited: "Limited autonomy",
  }[state.settings.mode];
  return (
    <div className="app">
      <aside>
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("overview");
          }}
        >
          <span className="brand-mark">▱</span>FilePilot <em>AI</em>
        </a>
        <div className="workspace-label">YOUR PERSONAL FILE ASSISTANT</div>
        <nav aria-label="Main navigation">
          {pages.map((item) => (
            <button
              key={item.id}
              className={page === item.id ? "nav-item active" : "nav-item"}
              aria-current={page === item.id ? "page" : undefined}
              onClick={() => setPage(item.id)}
            >
              <span aria-hidden="true">{item.symbol}</span>
              {item.label}
              {item.id === "review" && pending > 0 && <b>{pending}</b>}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-status">
            <i />
            On your Mac only
          </div>
          <p>
            Your files stay local
            <br />
            Your control stays yours
          </p>
          <small>Development alpha</small>
        </div>
      </aside>
      <main>
        <header>
          <div>
            <div className="eyebrow">YOUR FILES YOUR MAC YOUR CONTROL</div>
            <h1>
              {page === "overview"
                ? needsSetup
                  ? "Welcome to FilePilot"
                  : "A calmer place for your files"
                : pages.find((p) => p.id === page)?.label}
            </h1>
            <p>
              {page === "overview"
                ? "Less sorting more space to focus"
                : page === "review"
                  ? "A few decisions only you can make"
                  : page === "folders"
                    ? "The places FilePilot is allowed to look after"
                    : page === "activity"
                      ? "Every change recorded with a way back"
                      : "Make FilePilot work your way"}
            </p>
          </div>
          <span className="pill">
            <i
              className={
                state.settings.monitoring && active.length
                  ? "dot"
                  : "dot paused"
              }
            />
            {state.settings.monitoring && active.length
              ? "Monitoring on"
              : "Monitoring paused"}
          </span>
        </header>
        {!native && (
          <div className="notice">
            Browser preview only
            <br />
            Install the macOS app to grant folder access and maintain files
          </div>
        )}
        {(busy || message || state.notice) && (
          <div className="notice" role="status" aria-live="polite">
            {busy ? "Working on your Mac" : message}
            {state.notice && <p>{state.notice}</p>}
          </div>
        )}
        {!loaded && native ? (
          <section>
            <p>Opening your workspace</p>
          </section>
        ) : (
          <>
            {page === "overview" && needsSetup ? (
              <Setup
                startup={startup}
                onStartup={setStartup}
                roots={state.roots}
                settings={state.settings}
                reuseExisting={state.preferences.reuse_existing}
                disabled={busy || !native}
                previews={previews}
                onSettings={(patch) => task(() => update(patch))}
                onStructure={(reuse) => task(() => structure(reuse))}
                onChoose={() => task(choose)}
                onPreview={() => task(prepare)}
                onStart={() => task(start)}
              />
            ) : null}
            {page === "overview" && !needsSetup && (
              <>
                <section className="hero">
                  <div>
                    <div className="eyebrow">A LITTLE LESS TO THINK ABOUT</div>
                    <h2>
                      {state.settings.monitoring
                        ? "Your folders have a helping hand"
                        : "Take a breather"}
                    </h2>
                    <p>
                      {state.settings.monitoring
                        ? `${modeName} is checking your approved folders in the background`
                        : "Background organization is paused until you resume"}
                      <br />
                      Existing collections and coding projects stay together
                    </p>
                    <div className="actions">
                      <button
                        className="primary"
                        disabled={busy || !native}
                        onClick={() =>
                          task(() =>
                            update({ monitoring: !state.settings.monitoring }),
                          )
                        }
                      >
                        {state.settings.monitoring
                          ? "Pause maintenance"
                          : "Resume maintenance"}
                      </button>
                      <button onClick={() => setPage("review")}>
                        Open review inbox
                      </button>
                    </div>
                  </div>
                  <div className="hero-art" aria-hidden="true">
                    <div className="folder-back" />
                    <div className="folder-front">✓</div>
                    <span className="orbit one">✧</span>
                    <span className="orbit two">✧</span>
                  </div>
                </section>
                <div className="stats">
                  <div>
                    <small>FOLDERS COVERED</small>
                    <b>{active.length}</b>
                    <span>With your permission</span>
                  </div>
                  <div>
                    <small>WAITING FOR YOU</small>
                    <b>{pending}</b>
                    <span>Suggestions to review</span>
                  </div>
                  <div>
                    <small>REVERSIBLE CHANGES</small>
                    <b>{applied}</b>
                    <span>Available in Activity</span>
                  </div>
                </div>
                <div className="dashboard-grid">
                  <section>
                    <div className="section-heading">
                      <h3>Your spaces</h3>
                      <button
                        className="quiet"
                        onClick={() => setPage("folders")}
                      >
                        Manage folders
                      </button>
                    </div>
                    <div className="coverage">
                      {state.roots.map((r) => (
                        <div key={r.id}>
                          <span className="folder-icon">▱</span>
                          <span>
                            <b>{folderName(r.path)}</b>
                            <small>
                              {r.active
                                ? r.initial_reviewed
                                  ? "Setup complete"
                                  : "First preview needed"
                                : "Access needs renewal"}
                            </small>
                          </span>
                          <i className={r.active ? "dot" : "dot paused"} />
                        </div>
                      ))}
                    </div>
                  </section>
                  <section className="reassurance">
                    <span className="shield">◇</span>
                    <h3>Care comes built in</h3>
                    <p>
                      {protectedCount} protected entries in the last scans
                      <br />
                      No overwriting existing files
                      <br />
                      Trash always asks you first
                      <br />
                      Undo is in your activity log
                    </p>
                    <small>
                      Works while your Mac is awake
                      <br />
                      Pauses during sleep and reconciles on wake
                    </small>
                  </section>
                </div>
              </>
            )}
            {page === "review" && (
              <section>
                <div className="toolbar">
                  <label>
                    Folder{" "}
                    <select
                      aria-label="Review folder"
                      disabled={busy}
                      value={root?.id || ""}
                      onChange={(e) => {
                        setSelected(Number(e.target.value));
                        setDuplicateGroups([]);
                      }}
                    >
                      {state.roots.map((r) => (
                        <option key={r.id} value={r.id}>
                          {folderName(r.path)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    disabled={busy || !root?.active || !native}
                    onClick={() => task(rescan)}
                  >
                    Refresh now
                  </button>
                </div>
                {inventory ? (
                  <Review
                    key={inventory.root_id}
                    inventory={inventory}
                    busy={busy}
                    learn={state.settings.personalization}
                    trash={state.settings.trash_proposals}
                    onTask={task}
                    onMessage={setMessage}
                    onChanged={refresh}
                  />
                ) : (
                  <div className="empty">
                    <span>▤</span>
                    <h3>
                      {root
                        ? "Your inbox is getting ready"
                        : "Choose your spaces first"}
                    </h3>
                    <p>
                      {root
                        ? "Background scans will bring new suggestions here"
                        : "Complete setup to start maintaining folders"}
                    </p>
                  </div>
                )}
              </section>
            )}
            {page === "collections" && (
              <>
                <div className="toolbar">
                  <label>
                    Folder{" "}
                    <select
                      aria-label="Collection source folder"
                      value={root?.id || ""}
                      disabled={busy}
                      onChange={(e) => setSelected(Number(e.target.value))}
                    >
                      {active.map((r) => (
                        <option key={r.id} value={r.id}>
                          {folderName(r.path)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    disabled={busy || !native || !root?.active}
                    onClick={() => task(rescan)}
                  >
                    Refresh files
                  </button>
                </div>
                <Collections
                  key={root?.id || "none"}
                  root={root}
                  inventory={inventory}
                  busy={busy}
                  native={native}
                  trash={state.settings.trash_proposals}
                  onTask={task}
                  onMessage={setMessage}
                  onChanged={async () => {
                    await rescan();
                    await refresh();
                  }}
                />
              </>
            )}
            {page === "folders" && (
              <>
                <section>
                  <div className="section-heading">
                    <div>
                      <h3>Permission with a clear boundary</h3>
                      <p>
                        Choose folders once and FilePilot remembers their access
                        <br />
                        macOS may ask again if access changes
                      </p>
                    </div>
                    <button
                      className="primary"
                      disabled={busy || !native}
                      onClick={() =>
                        task(async () => {
                          await choose();
                          setPage("overview");
                        })
                      }
                    >
                      Choose folders
                    </button>
                  </div>
                  <div className="coverage">
                    {state.roots.map((r) => (
                      <div key={r.id}>
                        <span className="folder-icon">▱</span>
                        <span>
                          <b>{folderName(r.path)}</b>
                          <small>{r.path}</small>
                          <small>
                            {r.active
                              ? r.initial_reviewed
                                ? "Setup complete"
                                : "First preview needed"
                              : "Access needed"}
                          </small>
                        </span>
                        <button
                          disabled={busy || !r.active}
                          onClick={() =>
                            task(async () => {
                              await invoke("revoke", { rootId: r.id });
                              setPreviews([]);
                              setDuplicateGroups([]);
                              setMessage("Folder access revoked in FilePilot");
                            })
                          }
                        >
                          Revoke access
                        </button>
                      </div>
                    ))}
                  </div>
                  <p className="muted">
                    FilePilot covers your selected folders rather than every
                    Finder location
                    <br />
                    System folders app bundles cloud placeholders and project
                    contents are protected
                    <br />
                    Overlapping folder grants cannot be added together
                  </p>
                </section>
                <section>
                  <h3>Duplicate check</h3>
                  <p>
                    Identical contents are candidates for review
                    <br />
                    Nothing is removed automatically
                  </p>
                  <label>
                    Folder{" "}
                    <select
                      disabled={busy}
                      aria-label="Duplicate folder"
                      value={root?.id || ""}
                      onChange={(e) => {
                        setSelected(Number(e.target.value));
                        setDuplicateGroups([]);
                      }}
                    >
                      {active.map((r) => (
                        <option key={r.id} value={r.id}>
                          {folderName(r.path)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    disabled={busy || !root?.active || !native}
                    onClick={() =>
                      task(async () => {
                        const result = await invoke<{
                          groups: string[][];
                          partial: boolean;
                        }>("duplicates", { rootId: root.id });
                        setDuplicateGroups(result.groups);
                        setMessage(
                          result.partial
                            ? "Partial duplicate check within safety limits"
                            : "Duplicate check complete no deletion authorized",
                        );
                      })
                    }
                  >
                    Find duplicate files
                  </button>
                  {duplicateGroups.map((group, i) => (
                    <div className="plan-summary" key={i}>
                      {group.map((path) => (
                        <div className="duplicate-row" key={path}>
                          <span>{path}</span>
                          <button
                            disabled={busy || !state.settings.trash_proposals}
                            onClick={() =>
                              task(async () => {
                                const result = await invoke<number | null>(
                                  "trash_file",
                                  { rootId: root?.id, relativePath: path },
                                );
                                if (result !== null) setDuplicateGroups([]);
                                setMessage(
                                  result === null
                                    ? "Trash cancelled no file moved"
                                    : "Moved to Trash after separate confirmation",
                                );
                              })
                            }
                          >
                            Review Trash action
                          </button>
                        </div>
                      ))}
                    </div>
                  ))}
                </section>
              </>
            )}
            {page === "activity" && (
              <section>
                <div className="section-heading">
                  <div>
                    <h3>A clear trail of every change</h3>
                    <p>
                      Undo checks that files have not changed and never
                      overwrites another file
                    </p>
                  </div>
                  <button
                    disabled={busy || !native}
                    onClick={() =>
                      task(async () => {
                        await invoke("reconcile");
                        setMessage(
                          "Interrupted operations reconciled ambiguous cases remain blocked",
                        );
                      })
                    }
                  >
                    Check interrupted changes
                  </button>
                </div>
                {!state.history.length ? (
                  <div className="empty">
                    <span>↶</span>
                    <h3>No file operations yet</h3>
                    <p>Your first changes will appear here</p>
                  </div>
                ) : (
                  state.history.map((op) => (
                    <div className="activity-row" key={op.id}>
                      <span className="file-icon">
                        {op.kind === "mkdir" ? "▱" : "↗"}
                      </span>
                      <div>
                        <b>
                          {op.kind === "mkdir"
                            ? "Prepared folder"
                            : folderName(op.source)}
                        </b>
                        <small>
                          {op.source && `${op.source} → `}
                          {op.kind === "trash" ? "Trash" : op.destination}
                        </small>
                        <span className="pill">
                          {op.state.replaceAll("_", " ")}
                        </span>
                      </div>
                      {op.state === "applied" && op.kind !== "mkdir" && (
                        <button
                          disabled={busy}
                          onClick={() =>
                            task(async () => {
                              await invoke("undo", { operationId: op.id });
                              setMessage(
                                "Operation undone without overwriting another file",
                              );
                            })
                          }
                        >
                          Undo
                        </button>
                      )}
                    </div>
                  ))
                )}
              </section>
            )}
            {page === "preferences" && (
              <>
                <section>
                  <h3>Your everyday preferences</h3>
                  <Preferences
                    settings={state.settings}
                    reuseExisting={state.preferences.reuse_existing}
                    disabled={busy || !native}
                    onChange={(patch) => task(() => update(patch))}
                    onStructure={(reuse) => task(() => structure(reuse))}
                  />
                </section>
                <section>
                  <h3>Start with your Mac</h3>
                  <p>
                    Enable the login item once so FilePilot can resume after you
                    sign in
                    <br />
                    Closing the window keeps the app running
                    <br />
                    Quitting stops the agent
                  </p>
                  <div className="actions">
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
                </section>
                <section>
                  <details>
                    <summary>Personal rules and learning</summary>
                    <p>
                      Use an existing relative folder inside the selected grant
                    </p>
                    <label>
                      Folder{" "}
                      <select
                        disabled={busy}
                        aria-label="Rule folder"
                        value={root?.id || ""}
                        onChange={(e) => {
                          setSelected(Number(e.target.value));
                          setDuplicateGroups([]);
                        }}
                      >
                        {active.map((r) => (
                          <option key={r.id} value={r.id}>
                            {folderName(r.path)}
                          </option>
                        ))}
                      </select>
                    </label>
                    <div className="toolbar">
                      <select
                        aria-label="Category"
                        value={category}
                        onChange={(e) => setCategory(e.target.value)}
                      >
                        {categories.map((c) => (
                          <option key={c}>{c}</option>
                        ))}
                      </select>
                      <input
                        aria-label="Destination folder"
                        value={destination}
                        onChange={(e) => setDestination(e.target.value)}
                      />
                      {[false, true].map((rule) => (
                        <button
                          key={String(rule)}
                          disabled={
                            busy ||
                            !root?.active ||
                            (!rule && !state.settings.personalization)
                          }
                          onClick={() =>
                            task(async () => {
                              await invoke("correction", {
                                rootId: root?.id,
                                category,
                                destination,
                                rule,
                              });
                              setMessage(
                                rule
                                  ? "Limited autonomy rule explicitly authorized"
                                  : "Explicit local correction saved",
                              );
                            })
                          }
                        >
                          {rule ? "Authorize rule" : "Save correction"}
                        </button>
                      ))}
                    </div>
                    <div className="actions">
                      <button
                        disabled={busy || !native}
                        onClick={() =>
                          task(async () => {
                            await invoke("clear_learning");
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
                            setMessage("Approved rules cleared");
                          })
                        }
                      >
                        Clear authorized rules
                      </button>
                    </div>
                  </details>
                </section>
                <p className="footer-note">
                  All analysis stays on your Mac
                  <br />
                  Local semantic suggestions need review and do not grant
                  permission to move files
                  <br />
                  Permanent deletion is unavailable
                </p>
              </>
            )}
          </>
        )}
      </main>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
