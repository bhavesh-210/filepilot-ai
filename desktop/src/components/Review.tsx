import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Inventory, Proposal, Task, folderName } from "../types";
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
}: {
  inventory: Inventory;
  busy: boolean;
  learn: boolean;
  trash: boolean;
  onTask: Task;
  onMessage: (text: string) => void;
  onChanged: () => Promise<unknown>;
}) {
  const [suggestions, setSuggestions] = useState<Record<number, Semantic>>({});
  const [overrides, setOverrides] = useState<Record<number, Proposal>>({});
  return (
    <>
      <div className="section-heading">
        <div>
          <h3>A home for every loose file</h3>
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
