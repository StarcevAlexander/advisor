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
}

export interface ComposerPayload {
  text: string;
  attachments: Attachment[];
}
