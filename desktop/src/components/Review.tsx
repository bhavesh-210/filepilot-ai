import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Inventory, Proposal, Task, folderName } from "../types";
type FileSummary = {
  supported: boolean;
  excerpts: string[];
  scope: string;
  partial: boolean;
  method: string;
  categories: string[];
  destinations: { folder: string; reason: string; similar_files: string[] }[];
  suggested_title: string;
  recommendation: string;
};
type Semantic = {
  available: boolean;
  reason?: string;
  model?: string;
  content_used?: boolean;
  ranking?: { category: string; score: number }[];
};
export function Review({
  inventory,
  busy,
  learn,
  trash,
  onTask,
  onMessage,
  onChanged,
  onCollection,
}: {
  onCollection: (source: string, title: string) => void;
  inventory: Inventory;
  busy: boolean;
  learn: boolean;
  trash: boolean;
  onTask: Task;
  onMessage: (text: string) => void;
  onChanged: () => Promise<unknown>;
}) {
  const [summaries, setSummaries] = useState<Record<number, FileSummary>>({});
  const [chosenFolders, setChosenFolders] = useState<Record<number, string>>(
    {},
  );
  const [suggestions, setSuggestions] = useState<Record<number, Semantic>>({});
  const [overrides, setOverrides] = useState<Record<number, Proposal>>({});
  return (
    <>
      <div className="section-heading">
        <div>
          <h3>A home for every file</h3>
          <p>Review uncertain suggestions and anything waiting for approval</p>
        </div>
        <span className="pill">{inventory.proposals.length} pending</span>
      </div>
      {inventory.errors.map((error, i) => (
        <p className="error" key={i}>
          {error}
        </p>
      ))}
      {inventory.proposals.length === 0 && (
        <div className="empty">
          <span>✓</span>
          <h3>Nothing waiting here</h3>
          <p>
            Approved folders are checked automatically while monitoring is on
          </p>
        </div>
      )}
      {inventory.proposals.map((original) => {
        const p = overrides[original.id] || original;
        return (
          <article className="review-card" key={p.id}>
            <div className="file-heading">
              <span className="file-icon">▤</span>
              <div>
                <h4>{folderName(p.source)}</h4>
                <small>{p.source}</small>
              </div>
              <span className={p.blocked || !p.certain ? "pill warm" : "pill"}>
                {p.blocked || !p.certain
                  ? "Needs your eye"
                  : "Ready to organise"}
              </span>
            </div>
            <div className="destination">
              <small>PROPOSED HOME</small>
              <b>{p.destination || "Choose a category after inspection"}</b>
              <p>{p.reason}</p>
            </div>
            {summaries[p.id] && (
              <div className="file-summary">
                <div className="section-heading">
                  <h4>
                    {summaries[p.id].supported
                      ? "What is in this file"
                      : "Content summary unavailable"}
                  </h4>
                  <span className="pill">
                    {summaries[p.id].partial
                      ? "Partial sample"
                      : "Local analysis"}
                  </span>
                </div>
                <small>{summaries[p.id].scope}</small>
                <p className="summary-method">{summaries[p.id].method}</p>
                <ul className="summary-excerpts">
                  {summaries[p.id].excerpts.map((line, i) => (
                    <li key={i}>{line}</li>
                  ))}
                </ul>
                <small>
                  Quoted passages from the file are untrusted data and never
                  authorize actions
                </small>
                <p>{summaries[p.id].recommendation}</p>
                {summaries[p.id].categories.length > 0 && (
                  <small>
                    Topics suggested from the sampled text{" "}
                    {summaries[p.id].categories.join("  ")}
                  </small>
                )}
                <h4>Folders with related files</h4>
                {summaries[p.id].destinations.length === 0 && (
                  <p>
                    No strong related folder match found in the bounded local
                    comparison
                  </p>
                )}
                {summaries[p.id].destinations.map((candidate) => (
                  <div className="related-folder" key={candidate.folder}>
                    <div>
                      <b>{candidate.folder}</b>
                      <small>{candidate.reason}</small>
                      {candidate.similar_files.slice(0, 3).map((path) => (
                        <small key={path}>{path}</small>
                      ))}
                    </div>
                    <button
                      disabled={busy}
                      onClick={() =>
                        onTask(async () => {
                          await invoke("review_folder", {
                            proposalId: p.id,
                            folder: candidate.folder,
                          });
                          await invoke("apply", { proposalId: p.id });
                          await onChanged();
                          onMessage(
                            "Moved to the reviewed folder use Activity to undo",
                          );
                        })
                      }
                    >
                      Move to {candidate.folder}
                    </button>
                  </div>
                ))}
                <div className="actions">
                  <button
                    disabled={busy}
                    onClick={() =>
                      onCollection(p.source, summaries[p.id].suggested_title)
                    }
                  >
                    Create a titled collection
                  </button>
                  <label className="field">
                    Choose another existing folder
                    <select
                      aria-label={`Destination for ${p.source}`}
                      disabled={busy}
                      value={chosenFolders[p.id] || ""}
                      onChange={(e) =>
                        setChosenFolders((v) => ({
                          ...v,
                          [p.id]: e.target.value,
                        }))
                      }
                    >
                      <option value="">Choose a folder</option>
                      {inventory.folders.map((folder) => (
                        <option key={folder} value={folder}>
                          {folder}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    disabled={busy || !chosenFolders[p.id]}
                    onClick={() =>
                      onTask(async () => {
                        await invoke("review_folder", {
                          proposalId: p.id,
                          folder: chosenFolders[p.id],
                        });
                        await invoke("apply", { proposalId: p.id });
                        await onChanged();
                        onMessage("Move recorded use Activity to undo");
                      })
                    }
                  >
                    Move to chosen folder
                  </button>
                </div>
              </div>
            )}
            {suggestions[p.id] && (
              <div className="semantic">
                <small>
                  {suggestions[p.id].available
                    ? suggestions[p.id].content_used
                      ? "Text inspected on this Mac"
                      : "Filename inspected on this Mac"
                    : suggestions[p.id].reason}
                </small>
                {suggestions[p.id].ranking?.map((c) => (
                  <button
                    key={c.category}
                    disabled={busy}
                    onClick={() =>
                      onTask(async () => {
                        const updated = await invoke<Proposal>(
                          "review_destination",
                          { proposalId: p.id, category: c.category },
                        );
                        setOverrides((v) => ({ ...v, [p.id]: updated }));
                        onMessage(
                          "Destination selected explicitly approve the move separately",
                        );
                      })
                    }
                  >
                    {c.category}
                  </button>
                ))}
              </div>
            )}
            <div className="actions">
              <button
                className="primary"
                disabled={busy}
                onClick={() =>
                  onTask(async () => {
                    const result = await invoke<FileSummary>("summarize_file", {
                      proposalId: p.id,
                    });
                    setSummaries((v) => ({ ...v, [p.id]: result }));
                    onMessage(
                      result.supported
                        ? "Local summary ready review the sampled content and choose what happens next"
                        : result.scope,
                    );
                  })
                }
              >
                Summarise
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  onTask(async () => {
                    await invoke("keep_here", { proposalId: p.id });
                    await onChanged();
                    onMessage(
                      "Kept in its current location unchanged files will not be flagged again",
                    );
                  })
                }
              >
                Keep here
              </button>
            </div>
            <div className="actions">
              <button
                className="primary"
                disabled={busy || p.blocked}
                onClick={() =>
                  onTask(async () => {
                    await invoke("apply", { proposalId: p.id });
                    await onChanged();
                    onMessage("Move recorded you can undo it in Activity");
                  })
                }
              >
                Approve this move
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  onTask(async () => {
                    const result = await invoke<Semantic>(
                      "semantic_suggestions",
                      { proposalId: p.id },
                    );
                    setSuggestions((v) => ({ ...v, [p.id]: result }));
                    onMessage(
                      result.available
                        ? "Local suggestions ready for your review"
                        : result.reason || "Local model unavailable",
                    );
                  })
                }
              >
                Inspect locally
              </button>
              <details className="more">
                <summary>More options</summary>
                <button
                  disabled={busy || p.blocked || !learn}
                  onClick={() =>
                    onTask(async () => {
                      await invoke("remember_example", { proposalId: p.id });
                      await onChanged();
                      onMessage(
                        "Explicit file example saved future nearest matches will still require review",
                      );
                    })
                  }
                >
                  Remember this destination
                </button>
                <button
                  disabled={busy || !trash}
                  onClick={() =>
                    onTask(async () => {
                      const operation = await invoke<number | null>(
                        "trash_file",
                        { rootId: inventory.root_id, relativePath: p.source },
                      );
                      if (operation !== null) await onChanged();
                      onMessage(
                        operation === null
                          ? "Trash cancelled no file moved"
                          : "Moved to Trash after separate confirmation",
                      );
                    })
                  }
                >
                  Review Trash action
                </button>
              </details>
            </div>
          </article>
        );
      })}
    </>
  );
}
