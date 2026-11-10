import { Settings, Mode } from "../types";
export function Preferences({
  settings,
  reuseExisting,
  disabled,
  onChange,
  onStructure,
}: {
  settings: Settings;
  reuseExisting: boolean;
  disabled: boolean;
  onChange: (patch: Partial<Settings>) => void;
  onStructure: (reuse: boolean) => void;
}) {
  return (
    <div className="preference-list">
      <label className="preference">
        <span>
          <b>How much should FilePilot handle</b>
          <small>Uncertain files always stay for review</small>
        </span>
        <select
          aria-label="Autonomy"
          value={settings.mode}
          disabled={disabled}
          onChange={(e) => onChange({ mode: e.target.value as Mode })}
        >
          <option value="full">Full autonomy</option>
          <option value="smart">Smart approval</option>
          <option value="limited">Limited autonomy</option>
        </select>
      </label>
      <label className="preference">
        <span>
          <b>Build on my folder structure</b>
          <small>
            Reuse clearly named folders and leave existing collections together
          </small>
        </span>
        <input
          type="checkbox"
          checked={reuseExisting}
          disabled={disabled}
          onChange={(e) => onStructure(e.target.checked)}
        />
      </label>
      {(
        ["rename", "monitoring", "personalization", "trash_proposals"] as const
      ).map((key) => (
        <label className="preference" key={key}>
          <span>
            <b>
              {
                {
                  rename: "Tidy spaces in filenames",
                  monitoring: "Maintain folders in the background",
                  personalization: "Remember my explicit corrections",
                  trash_proposals: "Offer duplicate cleanup",
                }[key]
              }
            </b>
            <small>
              {
                {
                  rename: "Replace spaces with underscores on proposed moves",
                  monitoring: "Pause during sleep and reconcile on wake",
                  personalization: "Your learning history stays on this Mac",
                  trash_proposals:
                    "Moving anything to Trash needs a separate confirmation",
                }[key]
              }
            </small>
          </span>
          <input
            type="checkbox"
            disabled={disabled}
            checked={settings[key]}
            onChange={(e) => onChange({ [key]: e.target.checked })}
          />
        </label>
      ))}
    </div>
  );
}
