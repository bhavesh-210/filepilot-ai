import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Snapshot,
  Task,
  Route,
  TransferPlan,
  categories,
  folderName,
} from "../types";
export type TransferSeed = {
  rootId: number;
  source: string;
  category?: string;
  targetRootId?: number;
  folder?: string;
};
const aliases: Record<string, string[]> = {
  Education: ["education", "college", "school"],
  Development: ["development", "code"],
  Professional: ["professional", "work"],
  Finance: ["finance", "finances"],
  Media: ["media", "photos", "pictures"],
  Software: ["software", "installers"],
  Personal: ["personal"],
};
export function Workspace({
  state,
  disabled,
  seed,
  focus,
  onChoose,
  onTask,
  onMessage,
  onChanged,
  onFocus,
  onReview,
}: {
  state: Snapshot;
  disabled: boolean;
  seed: TransferSeed | null;
  focus: number | null;
  onChoose: () => void;
  onTask: Task;
  onMessage: (text: string) => void;
  onChanged: () => Promise<unknown>;
  onFocus: (id: number | null) => void;
  onReview: (id: number) => void;
}) {
  const heldFolder = useRef<number | null>(null);
  useEffect(
    () => () => {
      if (heldFolder.current !== null)
        invoke("cancel_folder_transfer", { token: heldFolder.current }).catch(
          () => {},
        );
    },
    [],
  );
  const [wholeFolder, setWholeFolder] = useState(false);
  const active = state.roots.filter((r) => r.active),
    ready = active.filter((r) => r.initial_reviewed);
  const preferred = (category: string) =>
    ready.find(
      (r) =>
        r.id ===
        state.routes.find((rule) => rule.category === category)?.target_root_id,
    ) ||
    ready.find(
      (r) =>
        folderName(r.path).toLowerCase() ===
        (category === "Media"
          ? "pictures"
          : category === "Software"
            ? "downloads"
            : "documents"),
    ) ||
    ready[0];
  const [content, setContent] = useState(true),
    [layout, setLayout] = useState<(Route & { create: boolean })[] | null>(
      null,
    ),
    [category, setCategory] = useState("Finance"),
    [target, setTarget] = useState<number>(ready[0]?.id || 0),
    [folder, setFolder] = useState("Finance"),
    [create, setCreate] = useState(true),
    [sourceRoot, setSourceRoot] = useState(seed?.rootId || ready[0]?.id || 0),
    [source, setSource] = useState(seed?.source || ""),
    [transferTarget, setTransferTarget] = useState(
      seed?.targetRootId || preferred(seed?.category || "Education")?.id || 0,
    ),
    [transferFolder, setTransferFolder] = useState(seed?.folder || ""),
    [plan, setPlan] = useState<TransferPlan | null>(null);
  const inventories = state.inventories;
  const sourceInventory = inventories.find((i) => i.root_id === sourceRoot);
  const destinationInventory = inventories.find(
    (i) => i.root_id === transferTarget,
  );
  const location = (id: number) =>
    folderName(state.roots.find((r) => r.id === id)?.path || "Location");
  const visible = state.directories.filter(
    (d) => focus === null || d.root_id === focus,
  );
  function suggested() {
    setLayout(
      categories.flatMap((category) => {
        const root = preferred(category);
        if (!root) return [];
        const inv = inventories.find((i) => i.root_id === root.id);
        const candidates = (inv?.folders || []).filter((path) =>
          (aliases[category] || []).includes(folderName(path).toLowerCase()),
        );
        const exact = candidates.find(
          (path) => folderName(path).toLowerCase() === category.toLowerCase(),
        );
        const folder = exact || candidates[0] || category;
        return [
          {
            category,
            target_root_id: root.id,
            folder,
            content,
            create: !inv?.folders.includes(folder),
          },
        ];
      }),
    );
  }
  return (
    <>
      <section>
        <div className="section-heading">
          <div>
            <h3>Your Mac by main directory</h3>
            <p>
              Every active location is monitored together
              <br />
              Recents is a Finder view and system locations are not organization
              targets
            </p>
          </div>
          <button className="primary" disabled={disabled} onClick={onChoose}>
            Choose main folders
          </button>
        </div>
        <div className="actions">
          <button
            disabled={disabled}
            onClick={() =>
              onTask(async () => {
                await invoke("scan_all");
                await onChanged();
                onMessage("All active locations refreshed");
              })
            }
          >
            Refresh all locations
          </button>
          <button className="quiet" onClick={() => onFocus(null)}>
            All locations
          </button>
        </div>
        <div className="directory-grid">
          {visible.map((d) => (
            <article className="directory-card" key={d.root_id}>
              <div className="section-heading">
                <span className="folder-icon">▱</span>
                <span className="pill">{d.directory}</span>
              </div>
              <h3>{location(d.root_id)}</h3>
              <small>{state.roots.find((r) => r.id === d.root_id)?.path}</small>
              <div className="mini-stats">
                <span>
                  <b>{d.files}</b> files
                </span>
                <span>
                  <b>{d.folders}</b> folders
                </span>
                <span>
                  <b>{d.pending}</b> waiting
                </span>
              </div>
              <p>
                {d.applied} recorded changes
                <br />
                {d.received} received from other locations
                <br />
                {d.sent} sent to other locations
              </p>
              <small>
                {d.complete
                  ? "Last scan complete"
                  : "Scan pending or incomplete"}
                <br />
                {d.protected} protected entries
              </small>
              <button onClick={() => onReview(d.root_id)}>
                Review this location
              </button>
            </article>
          ))}
        </div>
        {active.length === 0 && (
          <p>
            Choose Desktop Downloads Documents Pictures or other local folders
            through the macOS picker
          </p>
        )}
        <p className="muted">
          Choose several sibling folders with Command click
          <br />
          If Downloads contains an active test folder grant revoke that narrower
          grant in Your folders before choosing Downloads
          <br />
          Old operation history stays available
          <br />
          Cloud placeholders application bundles coding projects and protected
          system folders remain excluded
        </p>
      </section>
      <section>
        <div className="section-heading">
          <div>
            <h3>Approve a layout once</h3>
            <p>
              Saved rules route matching files across all approved locations
              <br />
              Full autonomy and Limited autonomy can execute saved rules
              <br />
              Smart approval always waits for an explicit move approval
            </p>
          </div>
          <span className="pill">{state.routes.length} saved rules</span>
        </div>
        <label className="preference">
          <span>
            <b>Include complete short text and PDF keyword matches</b>
            <small>
              Read supported text locally only when the complete file fits the
              limit
              <br />
              Require at least three topic terms and no competing category
              Complete PDFs must fit five pages and 2000 characters
              <br />
              These are deterministic rules rather than calibrated AI confidence
            </small>
          </span>
          <input
            type="checkbox"
            checked={content}
            disabled={disabled || !!layout}
            onChange={(e) => setContent(e.target.checked)}
          />
        </label>
        <button disabled={disabled || ready.length === 0} onClick={suggested}>
          Preview recommended layout
        </button>
        {layout && (
          <div className="plan-summary">
            <h4>Recommended main directory layout</h4>
            <p>
              These rules apply to current and future ordinary files including
              nested files in all approved locations
              <br />
              New folders listed below will be created when you approve
              <br />
              Nothing is overwritten or sent to Trash by these rules
            </p>
            <div className="table">
              <table>
                <thead>
                  <tr>
                    <th>Category</th>
                    <th>Destination</th>
                    <th>Folder</th>
                  </tr>
                </thead>
                <tbody>
                  {layout.map((rule) => (
                    <tr key={rule.category}>
                      <td>{rule.category}</td>
                      <td>
                        {location(rule.target_root_id)} / {rule.folder}
                      </td>
                      <td>
                        {rule.create ? "Create on approval" : "Use existing"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <button
              className="primary"
              disabled={disabled}
              onClick={() =>
                onTask(async () => {
                  let saved = 0;
                  try {
                    for (const rule of layout) {
                      const { create, ...route } = rule;
                      await invoke("save_route", { route, create });
                      saved++;
                    }
                    setLayout(null);
                    await invoke("scan_all");
                    onMessage(
                      `${saved} routing rules saved matching files will follow your autonomy mode`,
                    );
                  } finally {
                    await onChanged();
                  }
                })
              }
            >
              Approve this routing layout
            </button>
            <button disabled={disabled} onClick={() => setLayout(null)}>
              Cancel layout
            </button>
          </div>
        )}
        <details>
          <summary>Custom category destinations</summary>
          <div className="toolbar">
            <select
              aria-label="Routing category"
              value={category}
              disabled={disabled}
              onChange={(e) => {
                setCategory(e.target.value);
                setFolder(e.target.value);
              }}
            >
              {categories.map((c) => (
                <option key={c}>{c}</option>
              ))}
            </select>
            <select
              aria-label="Routing location"
              value={target}
              disabled={disabled}
              onChange={(e) => setTarget(Number(e.target.value))}
            >
              {ready.map((r) => (
                <option key={r.id} value={r.id}>
                  {location(r.id)}
                </option>
              ))}
            </select>
            <input
              aria-label="Routing folder"
              value={folder}
              disabled={disabled}
              placeholder="Relative folder or blank for location root"
              onChange={(e) => setFolder(e.target.value)}
            />
            <label>
              <input
                type="checkbox"
                checked={create}
                disabled={disabled}
                onChange={(e) => setCreate(e.target.checked)}
              />
              Create a single category folder if needed
            </label>
            <button
              disabled={disabled || !target}
              onClick={() =>
                onTask(async () => {
                  await invoke("save_route", {
                    route: {
                      category,
                      target_root_id: target,
                      folder,
                      content,
                    },
                    create: create && !!folder,
                  });
                  await invoke("scan_all");
                  onMessage("Routing destination saved");
                  await onChanged();
                })
              }
            >
              Save routing rule
            </button>
          </div>
        </details>
        {state.routes.length > 0 && (
          <>
            <div className="table">
              <table>
                <thead>
                  <tr>
                    <th>Saved category rule</th>
                    <th>Destination</th>
                    <th>Evidence allowed</th>
                  </tr>
                </thead>
                <tbody>
                  {state.routes.map((r) => (
                    <tr key={r.category}>
                      <td>{r.category}</td>
                      <td>
                        {location(r.target_root_id)} /{" "}
                        {r.folder || "location root"}
                      </td>
                      <td>
                        {r.content
                          ? "Filename or complete short text and PDF rule"
                          : "Filename category only"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <button
              disabled={disabled}
              onClick={() =>
                onTask(async () => {
                  await invoke("clear_routes");
                  await onChanged();
                  onMessage("Saved routing rules cleared");
                })
              }
            >
              Clear routing rules
            </button>
          </>
        )}
      </section>
      <section>
        <h3>Move between main locations</h3>
        <p>
          Preview an exact source and destination before moving
          <br />
          Both locations must be authorized and set up on the same disk
        </p>
        <label className="field">
          What to move
          <select
            aria-label="Transfer item type"
            disabled={disabled || !!plan}
            value={wholeFolder ? "folder" : "file"}
            onChange={(e) => {
              setWholeFolder(e.target.value === "folder");
              setSource("");
            }}
          >
            <option value="file">One file</option>
            <option value="folder">Whole ordinary folder</option>
          </select>
        </label>
        <p className="muted">
          Whole folder moves preserve nested structure and require a separate
          preview
          <br />
          Folders containing coding projects protected entries links or unstable
          files are rejected
          <br />
          Folder moves are limited to 100 files and 256 MiB and are never
          automatic
        </p>
        <div className="transfer-form">
          <label className="field">
            From location
            <select
              aria-label="Transfer source location"
              disabled={disabled || !!plan}
              value={sourceRoot}
              onChange={(e) => {
                setSourceRoot(Number(e.target.value));
                setSource("");
              }}
            >
              {ready.map((r) => (
                <option key={r.id} value={r.id}>
                  {location(r.id)}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            {wholeFolder ? "Folder" : "File"}
            <select
              aria-label={
                wholeFolder ? "Transfer source folder" : "Transfer source file"
              }
              disabled={disabled || !!plan}
              value={source}
              onChange={(e) => setSource(e.target.value)}
            >
              <option value="">
                {wholeFolder ? "Choose a folder" : "Choose a file"}
              </option>
              {(wholeFolder
                ? sourceInventory?.folders
                : sourceInventory?.items
              )?.map((path) => (
                <option key={path}>{path}</option>
              ))}
            </select>
          </label>
          <label className="field">
            To location
            <select
              aria-label="Transfer target location"
              disabled={disabled || !!plan}
              value={transferTarget}
              onChange={(e) => {
                setTransferTarget(Number(e.target.value));
                setTransferFolder("");
              }}
            >
              {ready.map((r) => (
                <option key={r.id} value={r.id}>
                  {location(r.id)}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            To folder
            <select
              aria-label="Transfer destination folder"
              disabled={disabled || !!plan}
              value={transferFolder}
              onChange={(e) => setTransferFolder(e.target.value)}
            >
              <option value="">Location root</option>
              {destinationInventory?.folders.map((path) => (
                <option key={path}>{path}</option>
              ))}
            </select>
          </label>
        </div>
        <button
          disabled={
            disabled || !source || !sourceRoot || !transferTarget || !!plan
          }
          onClick={() =>
            onTask(async () => {
              const prepared = await invoke<TransferPlan>(
                wholeFolder ? "prepare_folder_transfer" : "prepare_transfer",
                {
                  sourceRootId: sourceRoot,
                  source,
                  targetRootId: transferTarget,
                  folder: transferFolder,
                },
              );
              heldFolder.current = prepared.folder_transfer
                ? prepared.token
                : null;
              setPlan(prepared);
              onMessage("Transfer preview ready nothing moved");
            })
          }
        >
          Preview transfer
        </button>
        {plan && (
          <div className="plan-summary">
            <b>
              {location(plan.source_root_id)} / {plan.source}
            </b>
            {plan.folder_transfer && (
              <p>
                {plan.files_count} files and {plan.folders_count} folders will
                move together with their structure preserved
              </p>
            )}
            <p>
              Move to {location(plan.target_root_id)} / {plan.destination}
            </p>
            <div className="actions">
              <button
                className="primary"
                disabled={disabled}
                onClick={() =>
                  onTask(async () => {
                    await invoke(
                      plan.folder_transfer
                        ? "apply_folder_transfer"
                        : "apply_transfer",
                      { token: plan.token },
                    );
                    heldFolder.current = null;
                    setPlan(null);
                    await invoke("scan_all");
                    await onChanged();
                    onMessage("Transfer recorded use Activity to undo");
                  })
                }
              >
                Confirm transfer
              </button>
              <button
                disabled={disabled}
                onClick={() =>
                  onTask(async () => {
                    if (plan.folder_transfer)
                      await invoke("cancel_folder_transfer", {
                        token: plan.token,
                      });
                    heldFolder.current = null;
                    setPlan(null);
                  })
                }
              >
                Edit transfer
              </button>
            </div>
          </div>
        )}
      </section>
      {state.transfers.length > 0 && (
        <section>
          <h3>Ready under your saved rules</h3>
          <p>
            {state.transfers.length} planned routes remain visible here
            <br />
            Full autonomy can apply eligible routes automatically
            <br />
            Unmatched files stay safely in the review inbox
          </p>
          {state.transfers.slice(0, 24).map((p) => (
            <div className="related-folder" key={p.token}>
              <div>
                <b>
                  {location(p.source_root_id)} / {p.source}
                </b>
                <small>
                  To {location(p.target_root_id)} / {p.destination}
                </small>
                <small>{p.reason}</small>
              </div>
              <button
                disabled={disabled}
                onClick={() =>
                  onTask(async () => {
                    await invoke("apply_transfer", { token: p.token });
                    await invoke("scan_all");
                    await onChanged();
                    onMessage("Saved route applied use Activity to undo");
                  })
                }
              >
                Approve this route
              </button>
            </div>
          ))}
        </section>
      )}
    </>
  );
}
