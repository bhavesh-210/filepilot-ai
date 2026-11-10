export type Mode = "full" | "smart" | "limited";
export type Settings = {
  mode: Mode;
  rename: boolean;
  monitoring: boolean;
  personalization: boolean;
  trash_proposals: boolean;
  revision: number;
};
export type Folder = {
  id: number;
  path: string;
  active: boolean;
  initial_reviewed: boolean;
};
export type Proposal = {
  id: number;
  root_id: number;
  source: string;
  destination: string | null;
  category: string;
  reason: string;
  blocked: boolean;
  rename: boolean;
  certain: boolean;
};
export type Inventory = {
  root_id: number;
  files: number;
  folders: string[];
  protected: number;
  errors: string[];
  proposals: Proposal[];
};
export type Preview = {
  token: number;
  inventory: Inventory;
  create_folders: string[];
};
export type Operation = {
  id: number;
  root_id: number;
  source: string;
  destination: string;
  state: string;
  kind: string;
};
export type Snapshot = {
  settings: Settings;
  preferences: { reuse_existing: boolean };
  roots: Folder[];
  inventories: Inventory[];
  history: Operation[];
  notice: string;
};
export type Task = (action: () => Promise<unknown>) => Promise<void>;
export const categories = [
  "Education",
  "Development",
  "Professional",
  "Finance",
  "Media",
  "Software",
  "Personal",
];
export const folderName = (path: string) =>
  path.split("/").filter(Boolean).at(-1) || path;
