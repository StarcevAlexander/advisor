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
}

export type ChatMode = 'ask' | 'edit';

export interface StoredMessage {
  id: string;
  role: MessageRole;
  text: string;
  createdAt: string;
  edit?: boolean;
  link?: string;
}

export interface ComposerPayload {
  text: string;
  attachments: Attachment[];
}
