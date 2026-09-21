import type { Project, Task } from "./types";
import type { DeliveryWorkflow } from "./asset-delivery";
import type { AssetWork } from "./asset-work";

export type Point = [number, number];
export interface AssetFrame {
  id: string;
  sessionId: string;
  generation: string;
  sceneRevision: number;
  viewRevision: number;
  capturedAt: number;
  width: number;
  height: number;
  viewMatrix: number[];
  projectionMatrix: number[];
}
export interface ObserverView {
  target: [number, number, number];
  yaw: number;
  pitch: number;
  distance: number;
  width: number;
  height: number;
  seq: number;
}
export interface ObserverStatus {
  connected: boolean;
  taskId: string;
  projectId?: string;
  sessionId?: string;
  generation?: string;
  sceneRevision?: number;
  viewRevision?: number;
  view?: ObserverView;
  frame?: AssetFrame | null;
  busy?: { operation: string; startedAt: number } | null;
  error?: string | null;
  mainThreadAt?: number;
}
export interface AssetPick {
  frameId: string;
  objectId: string;
  objectName: string;
  instanceId: string;
  local: [number, number, number];
  world: [number, number, number];
  normal: [number, number, number];
  face: number;
  vertices: number;
  polygons: number;
  point: Point;
}
export interface AssetReference {
  id: string;
  taskId: string;
  projectId: string;
  frame: AssetFrame;
  pick: AssetPick | null;
  sha256: string;
  used: boolean;
}
export interface AssetAnnotation {
  kind: "box" | "arrow" | "brush";
  points: Point[];
}
export interface AssetStage {
  id: string;
  name: string;
  status: string;
  dependencies: string[];
  objects: string[];
  evidence: string;
  round: number;
}
export interface AssetSubmission {
  id: string;
  feedbackId: string;
  referenceId: string;
  text: string;
  timing: "now" | "afterRound";
  annotations: AssetAnnotation[];
}
export interface AssetFeedback {
  id: string;
  sourceTaskId: string;
  taskId: string;
  timing: AssetSubmission["timing"];
  text: string;
  reference: AssetReference;
  annotations: AssetAnnotation[];
  round: number;
  status: string;
  createdAt: string;
  deliveredAt: string | null;
  impact: string;
  affectedStages: string[];
  resumeStages: string[];
  imageObservation: string;
  evidence: string;
  checkpoint: string | null;
  history: { at: string; status: string; evidence?: string }[];
}
export interface AssetTaskState {
  delivery?: DeliveryWorkflow | null;
  work?: AssetWork;
  protocolVersion: number;
  taskId: string;
  projectId: string;
  revision: number;
  round: number;
  phase: string;
  sessionId: string | null;
  stages: AssetStage[];
  feedback: AssetFeedback[];
  checkpoint: string | null;
  lastFrame: AssetReference | null;
  recovery: string | null;
}
export interface AssetWindowState {
  task: Task;
  tasks: Task[];
  project: Project;
  asset: AssetTaskState;
}
