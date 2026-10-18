import { ChangeState, MessageRole } from './chat.models';

export interface ApiTurn {
  role: MessageRole;
  text: string;
}

export interface ApiReply {
  reply: string;
}

export interface ChangeReply {
  id: string;
  message: string;
  url: string;
}

export interface ChangePlanReply {
  summary: string;
  steps: string[];
}

export interface ChangeStatus {
  state: Exclude<ChangeState, 'unknown' | 'plan' | 'cancelled'>;
  pullUrl?: string;
  pullNumber?: number;
  runUrl?: string;
}

export interface ClientErrorReport {
  message: string;
  where: string;
  stack: string;
  version: string;
}

export interface ApiErrorBody {
  error?: string;
}

export class ChatApiError extends Error {
  constructor(
    message: string,
    readonly unauthorized = false,
  ) {
    super(message);
    this.name = 'ChatApiError';
  }
}
