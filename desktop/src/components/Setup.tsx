import { useState } from "react";
import { Folder, Preview, Settings, folderName } from "../types";
import { Preferences } from "./Preferences";
type Props = {
  startup: boolean;
  onStartup: (enabled: boolean) => void;
  roots: Folder[];
  settings: Settings;
  reuseExisting: boolean;
  disabled: boolean;
  previews: Preview[];
  onSettings: (patch: Partial<Settings>) => void;
  onStructure: (reuse: boolean) => void;
  onChoose: () => void;
  onPreview: () => void;
  onStart: () => void;
};
export function Setup(props: Props) {
  const [step, setStep] = useState(0);
  const pending = props.roots.filter((r) => r.active && !r.initial_reviewed);
  const ready =
    pending.length > 0 &&
    pending.every((r) =>
      props.previews.some((p) => p.inventory.root_id === r.id),
    );
  return (
    <section className="setup-card">
      <div className="eyebrow">MAKE YOURSELF AT HOME</div>
      <h2>
        A little setup
        <br />
        then less file admin
      </h2>
      <p>
        Choose your preferences once and let FilePilot look after the folders
        you allow
      </p>
      <ol className="steps">
        {["Your preferences", "Folder access", "First cleanup"].map(
          (label, i) => (
            <li key={label} aria-current={step === i ? "step" : undefined}>
              <span>{i + 1}</span>
              {label}
            </li>
          ),
        )}
      </ol>
      {step === 0 && (
        <>
          <Preferences
            settings={props.settings}
            reuseExisting={props.reuseExisting}
            disabled={props.disabled}
            onChange={props.onSettings}
            onStructure={props.onStructure}
          />
          <label className="preference">
            <span>
              <b>Start when I sign in</b>
              <small>macOS may ask you to approve the login item</small>
            </span>
            <input
              type="checkbox"
              checked={props.startup}
              disabled={props.disabled}
              onChange={(e) => props.onStartup(e.target.checked)}
            />
          </label>
          <div className="actions">
            <button
              className="primary"
              disabled={props.disabled}
              onClick={() => setStep(1)}
            >
              Continue to folder access
            </button>
          </div>
        </>
      )}
      {step === 1 && (
        <>
          <h3>Give FilePilot room to help</h3>
          <p>
            Choose several folders in the macOS picker with Command click
            <br />
            For this alpha start with disposable test folders
          </p>
          <div className="coverage">
            {props.roots.map((r) => (
              <div key={r.id}>
                <span className="folder-icon">▱</span>
                <span>
                  <b>{folderName(r.path)}</b>
                  <small>{r.path}</small>
                </span>
                <span className="pill">
                  {r.active ? "Access granted" : "Access needed"}
                </span>
              </div>
            ))}
          </div>
          <p className="muted">
            Only approved folders are covered
            <br />
            Projects protected folders and existing collections stay together
          </p>
          <div className="actions">
            <button disabled={props.disabled} onClick={props.onChoose}>
              Choose folders
            </button>
            <button
              className="primary"
              disabled={props.disabled || pending.length === 0}
              onClick={() => {
                setStep(2);
                props.onPreview();
              }}
            >
              Understand my folders
            </button>
            <button
              className="quiet"
              disabled={props.disabled}
              onClick={() => setStep(0)}
            >
              Back
            </button>
          </div>
        </>
      )}
      {step === 2 && (
        <>
          <h3>Your first cleanup preview</h3>
          <p>
            Nothing has been moved or created yet
            <br />
            Starting maintenance allows the folders below to be created and safe
            changes to follow your chosen mode
          </p>
          {props.previews.map((p) => (
            <div className="plan-summary" key={p.inventory.root_id}>
              <h4>
                {folderName(
                  props.roots.find((r) => r.id === p.inventory.root_id)?.path ||
                    "Folder",
                )}
              </h4>
              <div className="mini-stats">
                <span>
                  <b>{p.inventory.files}</b> files checked
                </span>
                <span>
                  <b>{p.inventory.folders.length}</b> folders mapped
                </span>
                <span>
                  <b>{p.inventory.protected}</b> protected entries
                </span>
              </div>
              <p>
                {p.create_folders.length
                  ? `Create only the folders needed for this plan ${p.create_folders.join("  ")}`
                  : "Use the available folders without creating extra categories"}
              </p>
              <details>
                <summary>See folder structure and proposed changes</summary>
                <ul className="structure">
                  {p.inventory.folders.map((f) => (
                    <li key={f}>▱ {f}</li>
                  ))}
                </ul>
                <div className="table">
                  <table>
                    <thead>
                      <tr>
                        <th>Current file</th>
                        <th>Proposed home</th>
                        <th>Decision</th>
                      </tr>
                    </thead>
                    <tbody>
                      {p.inventory.proposals.map((row) => (
                        <tr key={row.id}>
                          <td>{row.source}</td>
                          <td>{row.destination || "Keep for review"}</td>
                          <td>
                            {row.blocked || !row.certain
                              ? "Needs attention"
                              : "Ready under your chosen mode"}
                            <small>{row.reason}</small>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </details>
            </div>
          ))}
          <p className="muted">
            Full autonomy handles stable unambiguous files
            <br />
            Smart approval keeps every move in your inbox
            <br />
            Limited autonomy follows only your saved rules
            <br />
            New category folders may be created as needed in Full autonomy
            <br />
            Trash always asks separately
          </p>
          <div className="actions">
            <button
              className="primary"
              disabled={props.disabled || !ready}
              onClick={props.onStart}
            >
              Start maintaining my folders
            </button>
            <button disabled={props.disabled} onClick={props.onPreview}>
              Refresh preview
            </button>
            <button
              className="quiet"
              disabled={props.disabled}
              onClick={() => setStep(1)}
            >
              Back
            </button>
          </div>
        </>
      )}
    </section>
  );
}
