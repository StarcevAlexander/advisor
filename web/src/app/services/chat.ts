import { Service, inject, signal } from '@angular/core';
import { WELCOME_TEXT } from '../constants';
import { ApiTurn, ChatApiError, ChatMessage, ComposerPayload } from '../models';
import { Access } from './access';
import { ChatApi } from './chat-api';

@Service()
export class Chat {
  readonly messages = signal<ChatMessage[]>([
    {
      id: crypto.randomUUID(),
      role: 'assistant',
      text: WELCOME_TEXT,
      attachments: [],
      createdAt: new Date(),
    },
  ]);
  readonly busy = signal(false);

  private readonly api = inject(ChatApi);
  private readonly access = inject(Access);

  send(payload: ComposerPayload): void {
    const text = payload.text.trim();
    if (this.busy() || (!text && payload.attachments.length === 0)) {
      return;
    }

    const history = this.buildHistory();
    const pendingId = crypto.randomUUID();
    this.messages.update((messages) => [
      ...messages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        text,
        attachments: payload.attachments,
        createdAt: new Date(),
      },
      {
        id: pendingId,
        role: 'assistant',
        text: '',
        attachments: [],
        createdAt: new Date(),
        pending: true,
      },
    ]);
    this.busy.set(true);

    void this.requestReply(pendingId, { text, attachments: payload.attachments }, history);
  }

  remove(id: string): void {
    const target = this.messages().find((message) => message.id === id);
    if (!target || target.pending) {
      return;
    }
    target.attachments.forEach((attachment) => {
      if (attachment.previewUrl) {
        URL.revokeObjectURL(attachment.previewUrl);
      }
    });
    this.messages.update((messages) => messages.filter((message) => message.id !== id));
  }

  private async requestReply(
    pendingId: string,
    payload: ComposerPayload,
    history: ApiTurn[],
  ): Promise<void> {
    try {
      const reply = await this.api.ask(payload, history);
      this.resolvePending(pendingId, reply, false);
    } catch (error) {
      if (error instanceof ChatApiError && error.unauthorized) {
        this.access.clear();
      }
      const message = error instanceof ChatApiError ? error.message : String(error);
      this.resolvePending(pendingId, message, true);
    }
  }

  private buildHistory(): ApiTurn[] {
    return this.messages()
      .filter((message) => !message.pending && !message.failed && message.text.trim().length > 0)
      .map((message) => ({ role: message.role, text: message.text }));
  }

  private resolvePending(id: string, text: string, failed: boolean): void {
    this.messages.update((messages) =>
      messages.map((message) =>
        message.id === id ? { ...message, text, pending: false, failed } : message,
      ),
    );
    this.busy.set(false);
  }
}
