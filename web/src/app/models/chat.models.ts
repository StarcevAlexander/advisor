export type MessageRole = 'user' | 'assistant';

export interface Attachment {
  id: string;
  name: string;
  mimeType: string;
  size: number;
  file: File;
  previewUrl?: string;
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  text: string;
  attachments: Attachment[];
  createdAt: Date;
  pending?: boolean;
  failed?: boolean;
  edit?: boolean;
  link?: string;
  linkLabel?: string;
  changeId?: string;
  changeState?: ChangeState;
  plan?: ChangePlan;
  rollback?: boolean;
}

export type ChatMode = 'ask' | 'edit';

export interface PlanStep {
  id: string;
  text: string;
  selected: boolean;
}

export interface ChangePlan {
  request: string;
  steps: PlanStep[];
}

export type ChangeState =
  | 'plan'
  | 'cancelled'
  | 'queued'
  | 'working'
  | 'ready'
  | 'draft'
  | 'merged'
  | 'closed'
  | 'failed'
  | 'nochange'
  | 'unknown';

export interface StoredMessage {
  id: string;
  role: MessageRole;
  text: string;
  createdAt: string;
  edit?: boolean;
  link?: string;
  linkLabel?: string;
  changeId?: string;
  changeState?: ChangeState;
  plan?: ChangePlan;
  rollback?: boolean;
}

export interface ComposerPayload {
  text: string;
  attachments: Attachment[];
}

export interface ChatReply {
  id: string;
  text: string;
}
