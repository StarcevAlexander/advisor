import { Service } from '@angular/core';
import { CHANGE_STATES, CHAT_HISTORY_STORAGE_KEY, MAX_STORED_MESSAGES } from '../constants';
import { ChangePlan, ChangeState, ChatMessage, MessageRole, StoredMessage } from '../models';

const ROLES: readonly MessageRole[] = ['user', 'assistant'];

function isChangePlan(value: unknown): value is ChangePlan {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate['request'] === 'string' &&
    Array.isArray(candidate['steps']) &&
    candidate['steps'].every((step: unknown) => {
      if (typeof step !== 'object' || step === null) {
        return false;
      }
      const item = step as Record<string, unknown>;
      return (
        typeof item['id'] === 'string' &&
        typeof item['text'] === 'string' &&
        typeof item['selected'] === 'boolean'
      );
    })
  );
}

function isStoredMessage(value: unknown): value is StoredMessage {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate['id'] === 'string' &&
    typeof candidate['text'] === 'string' &&
    typeof candidate['createdAt'] === 'string' &&
    !Number.isNaN(Date.parse(candidate['createdAt'])) &&
    ROLES.includes(candidate['role'] as MessageRole) &&
    (candidate['link'] === undefined || typeof candidate['link'] === 'string') &&
    (candidate['edit'] === undefined || typeof candidate['edit'] === 'boolean') &&
    (candidate['linkLabel'] === undefined || typeof candidate['linkLabel'] === 'string') &&
    (candidate['changeId'] === undefined || typeof candidate['changeId'] === 'string') &&
    (candidate['changeState'] === undefined ||
      CHANGE_STATES.includes(candidate['changeState'] as ChangeState)) &&
    (candidate['plan'] === undefined || isChangePlan(candidate['plan']))
  );
}

@Service()
export class ChatHistory {
  load(): ChatMessage[] {
    try {
      const raw = localStorage.getItem(CHAT_HISTORY_STORAGE_KEY);
      if (!raw) {
        return [];
      }
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        return [];
      }
      return parsed.filter(isStoredMessage).map((stored) => ({
        id: stored.id,
        role: stored.role,
        text: stored.text,
        attachments: [],
        createdAt: new Date(stored.createdAt),
        edit: stored.edit,
        link: stored.link,
        linkLabel: stored.linkLabel,
        changeId: stored.changeId,
        changeState: stored.changeState,
        plan: stored.plan,
      }));
    } catch {
      return [];
    }
  }

  save(messages: ChatMessage[]): void {
    const stored: StoredMessage[] = messages
      .filter((message) => !message.pending && !message.failed && message.text.trim().length > 0)
      .slice(-MAX_STORED_MESSAGES)
      .map((message) => ({
        id: message.id,
        role: message.role,
        text: message.text,
        createdAt: message.createdAt.toISOString(),
        edit: message.edit,
        link: message.link,
        linkLabel: message.linkLabel,
        changeId: message.changeId,
        changeState: message.changeState,
        plan: message.plan,
      }));
    try {
      localStorage.setItem(CHAT_HISTORY_STORAGE_KEY, JSON.stringify(stored));
    } catch {
      return;
    }
  }
}
