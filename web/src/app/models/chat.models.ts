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
}

export type ChatMode = 'ask' | 'edit';

export type ChangeState =
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
}

export interface ComposerPayload {
  text: string;
  attachments: Attachment[];
}
