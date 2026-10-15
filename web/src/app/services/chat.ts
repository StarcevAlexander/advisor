import { Service, effect, inject, signal } from '@angular/core';
import { API_MESSAGES, WELCOME_TEXT } from '../constants';
import {
  ApiTurn,
  Attachment,
  ChatApiError,
  ChatMessage,
  ChatMode,
  ComposerPayload,
} from '../models';
import { Access } from './access';
import { ChatApi } from './chat-api';
import { ChatHistory } from './chat-history';

@Service()
export class Chat {
  private readonly api = inject(ChatApi);
  private readonly access = inject(Access);
  private readonly history = inject(ChatHistory);

  readonly messages = signal<ChatMessage[]>(this.restoreMessages());
  readonly busy = signal(false);
  readonly mode = signal<ChatMode>('ask');

  constructor() {
    effect(() => this.history.save(this.messages()));
  }

  toggleEditMode(): void {
    if (this.busy()) {
      return;
    }
    this.mode.update((mode) => (mode === 'edit' ? 'ask' : 'edit'));
  }

  clear(): void {
    if (this.busy()) {
      return;
    }
    this.messages().forEach((message) => this.revokePreviews(message));
    this.messages.set([this.createWelcomeMessage()]);
  }

  send(payload: ComposerPayload): void {
    const text = payload.text.trim();
    if (this.busy() || (!text && payload.attachments.length === 0)) {
      return;
    }

    const editing = this.mode() === 'edit';
    const history = this.buildHistory();
    const pendingId = this.appendExchange(text, payload.attachments, editing);

    if (editing) {
      this.mode.set('ask');
      void this.requestChange(pendingId, text, payload.attachments.length);
      return;
    }
    void this.requestReply(pendingId, { text, attachments: payload.attachments }, history);
  }

  remove(id: string): void {
    const target = this.messages().find((message) => message.id === id);
    if (!target || target.pending) {
      return;
    }
    this.revokePreviews(target);
    this.messages.update((messages) => messages.filter((message) => message.id !== id));
  }

  private appendExchange(text: string, attachments: Attachment[], edit: boolean): string {
    const pendingId = crypto.randomUUID();
    this.messages.update((messages) => [
      ...messages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        text,
        attachments,
        createdAt: new Date(),
        edit,
      },
      {
        id: pendingId,
        role: 'assistant',
        text: '',
        attachments: [],
        createdAt: new Date(),
        pending: true,
        edit,
      },
    ]);
    this.busy.set(true);
    return pendingId;
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
      this.failPending(pendingId, error);
    }
  }

  private async requestChange(
    pendingId: string,
    request: string,
    attachmentsCount: number,
  ): Promise<void> {
    try {
      if (attachmentsCount > 0) {
        throw new ChatApiError(API_MESSAGES.editNoFiles);
      }
      const reply = await this.api.requestChange(request);
      this.resolvePending(pendingId, reply.message, false, reply.url);
    } catch (error) {
      this.failPending(pendingId, error);
    }
  }

  private failPending(id: string, error: unknown): void {
    if (error instanceof ChatApiError && error.unauthorized) {
      this.access.clear();
    }
    const message = error instanceof ChatApiError ? error.message : String(error);
    this.resolvePending(id, message, true);
  }

  private restoreMessages(): ChatMessage[] {
    const restored = this.history.load();
    return restored.length > 0 ? restored : [this.createWelcomeMessage()];
  }

  private createWelcomeMessage(): ChatMessage {
    return {
      id: crypto.randomUUID(),
      role: 'assistant',
      text: WELCOME_TEXT,
      attachments: [],
      createdAt: new Date(),
    };
  }

  private revokePreviews(message: ChatMessage): void {
    message.attachments.forEach((attachment) => {
      if (attachment.previewUrl) {
        URL.revokeObjectURL(attachment.previewUrl);
      }
    });
  }

  private buildHistory(): ApiTurn[] {
    return this.messages()
      .filter(
        (message) =>
          !message.pending && !message.failed && !message.edit && message.text.trim().length > 0,
      )
      .map((message) => ({ role: message.role, text: message.text }));
  }

  private resolvePending(id: string, text: string, failed: boolean, link?: string): void {
    this.messages.update((messages) =>
      messages.map((message) =>
        message.id === id ? { ...message, text, pending: false, failed, link } : message,
      ),
    );
    this.busy.set(false);
  }
}
