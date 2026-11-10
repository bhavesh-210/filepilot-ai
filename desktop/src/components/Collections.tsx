import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Folder, Inventory, Proposal, Task, folderName } from "../types";
type Plan = {
  token: number;
  root_id: number;
  folder: string;
  create_folder: boolean;
  proposals: Proposal[];
};
export function Collections({
  root,
  inventory,
  busy,
  native,
  trash,
  onTask,
  onMessage,
  onChanged,
  initialSources = [],
  initialTitle = "",
}: {
  initialSources?: string[];
  initialTitle?: string;
  root: Folder | undefined;
  inventory: Inventory | undefined;
  busy: boolean;
  native: boolean;
  trash: boolean;
  onTask: Task;
  onMessage: (text: string) => void;
  onChanged: () => Promise<unknown>;
}) {
  const [selected, setSelected] = useState<string[]>(initialSources),
    [query, setQuery] = useState(""),
    [title, setTitle] = useState(initialTitle),
    [existing, setExisting] = useState(""),
    [create, setCreate] = useState(true),
    [plan, setPlan] = useState<Plan | null>(null);
  const held = useRef<Plan | null>(null);
  useEffect(
    () => () => {
      const current = held.current;
      if (current && native)
        invoke("cancel_collection", {
          rootId: current.root_id,
          token: current.token,
        }).catch(() => {});
    },
    [native],
  );
  const items = (inventory?.items || []).filter((path) =>
    path.toLowerCase().includes(query.toLowerCase()),
  );
  const disabled = busy || !native || !root?.active || !root.initial_reviewed;
  async function edit() {
    if (plan) {
      await invoke("cancel_collection", {
        rootId: plan.root_id,
        token: plan.token,
      });
      held.current = null;
      setPlan(null);
    }
  }
  return (
    <section>
      <div className="section-heading">
        <div>
          <h3>Keep the important things together</h3>
          <p>
            Select files you care about and give them a shared home
            <br />
            You decide what is important and what is unwanted
          </p>
        </div>
        <span className="pill">{selected.length} selected</span>
      </div>
      {!root?.initial_reviewed && (
        <p className="error">
          Complete first cleanup setup before arranging collections
        </p>
      )}
      <div className="collection-layout">
        <div>
          <label className="field">
            Find files
            <input
              aria-label="Find collection files"
              disabled={disabled || !!plan}
              placeholder="Search filenames or paths"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <div className="file-picker" aria-label="Files in this folder">
            {items.slice(0, 200).map((path) => (
              <label className="pick-file" key={path}>
                <input
                  type="checkbox"
                  aria-label={`Select ${path}`}
                  disabled={
                    disabled ||
                    !!plan ||
                    (!selected.includes(path) && selected.length >= 100)
                  }
                  checked={selected.includes(path)}
                  onChange={(e) =>
                    setSelected((current) =>
                      e.target.checked
                        ? [...current, path]
                        : current.filter((item) => item !== path),
                    )
                  }
                />
                <span>
                  <b>{folderName(path)}</b>
                  <small>{path}</small>
                </span>
              </label>
            ))}
            {items.length === 0 && <p>No matching files in the last scan</p>}
          </div>
          <small>
            Showing {Math.min(items.length, 200)} of {items.length} matching
            files
            <br />
            Select up to 100 files inside this approved folder
            <br />
            Projects and protected entries are excluded
          </small>
        </div>
        <div className="collection-destination">
          <label className="field">
            Where should they go
            <select
              aria-label="Collection destination type"
              value={create ? "new" : "existing"}
              disabled={disabled || !!plan}
              onChange={(e) => setCreate(e.target.value === "new")}
            >
              <option value="new">A new named folder</option>
              <option value="existing">An existing folder</option>
            </select>
          </label>
          {create ? (
            <label className="field">
              Folder title
              <input
                aria-label="Collection folder title"
                disabled={disabled || !!plan}
                value={title}
                placeholder="Suggest a title when I preview"
                onChange={(e) => setTitle(e.target.value)}
              />
              <small>The suggestion uses filenames and can be edited</small>
            </label>
          ) : (
            <label className="field">
              Choose a folder
              <select
                aria-label="Existing collection folder"
                disabled={disabled || !!plan}
                value={existing}
                onChange={(e) => setExisting(e.target.value)}
              >
                <option value="">Choose an existing folder</option>
                {inventory?.folders.map((path) => (
                  <option key={path} value={path}>
                    {path}
                  </option>
                ))}
              </select>
            </label>
          )}
          <button
            className="primary"
            disabled={
              disabled ||
              !!plan ||
              selected.length === 0 ||
              (!create && !existing)
            }
            onClick={() =>
              onTask(async () => {
                if (!root) return;
                const folder = create
                  ? title ||
                    (await invoke<string>("collection_title", {
                      rootId: root.id,
                      sources: selected,
                    }))
                  : existing;
                if (create) setTitle(folder);
                const next = await invoke<Plan>("plan_collection", {
                  rootId: root.id,
                  sources: selected,
                  folder,
                  createFolder: create,
                });
                held.current = next;
                setPlan(next);
                onMessage(
                  "Collection preview ready no files moved or folders created",
                );
              })
            }
          >
            Preview collection
          </button>
          <div className="trash-panel">
            <h4>Clear out unwanted files</h4>
            <p>
              Selected files can go to Trash
              <br />
              Every file gets a separate confirmation and can be restored
              through Activity
            </p>
            <button
              disabled={disabled || !!plan || selected.length === 0 || !trash}
              onClick={() =>
                onTask(async () => {
                  if (!root) return;
                  let moved = 0;
                  for (const path of selected) {
                    const operation = await invoke<number | null>(
                      "trash_file",
                      { rootId: root.id, relativePath: path },
                    );
                    if (operation === null) break;
                    moved++;
                    setSelected((current) =>
                      current.filter((item) => item !== path),
                    );
                  }
                  await onChanged();
                  onMessage(
                    moved
                      ? `${moved} files moved to Trash after confirmation use Activity to undo`
                      : "Trash cancelled no files moved",
                  );
                })
              }
            >
              Move selected to Trash
            </button>
          </div>
        </div>
      </div>
      {plan && (
        <div className="plan-summary">
          <h3>
            {plan.create_folder ? "Create" : "Use"} {plan.folder}
          </h3>
          <p>
            Review these exact moves before confirming
            <br />
            Filenames are preserved and nothing will be overwritten
          </p>
          <div className="table">
            <table>
              <thead>
                <tr>
                  <th>Current file</th>
                  <th>New home</th>
                </tr>
              </thead>
              <tbody>
                {plan.proposals.map((p) => (
                  <tr key={p.id}>
                    <td>{p.source}</td>
                    <td>{p.destination}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="actions">
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                onTask(async () => {
                  const result = await invoke<{
                    operations: number[];
                    errors: string[];
                  }>("apply_collection", {
                    rootId: plan.root_id,
                    token: plan.token,
                  });
                  held.current = null;
                  setPlan(null);
                  setSelected([]);
                  await onChanged();
                  onMessage(
                    result.errors.length
                      ? `${result.operations.length} moves verified before work stopped ${result.errors.join(" ")} Check Activity before trying again`
                      : `${result.operations.length} files added to ${plan.folder} use Activity to undo each move`,
                  );
                })
              }
            >
              Confirm collection
            </button>
            <button disabled={busy} onClick={() => onTask(edit)}>
              Edit selection
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
