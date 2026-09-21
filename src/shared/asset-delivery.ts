import type { AssetWork } from "./asset-work";

export interface DeliveryWorkflow {
  templateId: string;
  templateVersion: number;
  approved: string[];
  pending: string | null;
  history: string[];
}

export interface DeliveryCandidate {
  id: string;
  taskId: string;
  stageId: string;
  templateId: string;
  templateVersion: number;
  inputCandidates: string[];
  attemptIds?: string[];
  files: Record<string, string>;
  summary: string;
  threadId: string;
  turnId: string;
  sessionId: string | null;
  createdAt: string;
}

export type DeliveryDecision = "approve" | "reject" | "reopen";
export interface DeliveryState {
  revision: number;
  workflow: DeliveryWorkflow | null;
  work?: AssetWork;
  candidates: DeliveryCandidate[];
  decisions: {
    request: { candidateId: string; decision: DeliveryDecision; note: string };
    source: "owner";
    time: string;
    fileCheck: string;
  }[];
}
