import { Service, signal } from '@angular/core';
import { STUB_REPLY_DELAY_MS, WELCOME_TEXT } from '../constants';
import { ChatMessage, ComposerPayload } from '../models';

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

  send(payload: ComposerPayload): void {
    const text = payload.text.trim();
    if (this.busy() || (!text && payload.attachments.length === 0)) {
      return;
    }

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

    setTimeout(
      () => this.resolvePending(pendingId, this.buildStubReply(payload)),
      STUB_REPLY_DELAY_MS,
    );
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

  private resolvePending(id: string, text: string): void {
    this.messages.update((messages) =>
      messages.map((message) =>
        message.id === id ? { ...message, text, pending: false } : message,
      ),
    );
    this.busy.set(false);
  }

  private buildStubReply(payload: ComposerPayload): string {
    const files = payload.attachments.length;
    const filesNote = files > 0 ? ` Вложений: ${files}.` : '';
    return `Сервер советчика ещё не подключён, это тестовый ответ. Вы сказали: «${payload.text.trim() || 'без текста'}».${filesNote}`;
  }
}
