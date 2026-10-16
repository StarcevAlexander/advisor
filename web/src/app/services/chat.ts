import { Service, computed, effect, inject, signal } from '@angular/core';
import {
  API_MESSAGES,
  CHANGE_ACTIVE_LIMIT_MS,
  CHANGE_ACTIVE_POLL_INTERVAL_MS,
  CHANGE_ACTIVE_STATES,
  CHANGE_FIRST_POLL_DELAY_MS,
  CHANGE_LIST_LINK_LABEL,
  CHANGE_PLAN_STEPS_HEADER,
  CHANGE_REQUEST_MAX_RUNES,
  CHANGE_REQUEST_STEPS_HEADER,
  CHANGE_REVIEW_LIMIT_MS,
  CHANGE_REVIEW_POLL_INTERVAL_MS,
  CHANGE_RUN_LINK_LABEL,
  CHANGE_STATE_TEXT,
  CHANGE_TRACKED_STATES,
  ROLLBACK_STATE_TEXT,
  MAX_HISTORY_TURNS,
  WELCOME_TEXT,
} from '../constants';
import {
  ApiTurn,
  Attachment,
  ChangePlan,
  ChangeState,
  ChangeStatus,
  ChatApiError,
  ChatMessage,
  ChatMode,
  ChatReply,
  ComposerPayload,
} from '../models';
import { Access } from './access';
import { ChatApi } from './chat-api';
import { ChatHistory } from './chat-history';
import { Settings } from './settings';

@Service()
export class Chat {
  private readonly api = inject(ChatApi);
  private readonly access = inject(Access);
  private readonly history = inject(ChatHistory);
  private readonly settings = inject(Settings);

  readonly messages = signal<ChatMessage[]>(this.restoreMessages());
  readonly busy = signal(false);
  readonly mode = computed<ChatMode>(() => (this.settings.developerMode() ? 'edit' : 'ask'));
  readonly lastReply = signal<ChatReply | undefined>(undefined);
  readonly contextCount = computed(() =>
    Math.min(this.contextTurns(this.messages()).length, MAX_HISTORY_TURNS),
  );
  readonly contextTotal = computed(() => this.contextTurns(this.messages()).length);
  readonly retryableIds = computed(() => {
    const messages = this.messages();
    const ids = new Set<string>();
    messages.forEach((message, index) => {
      const reply = messages[index + 1];
      if (
        message.role === 'user' &&
        reply?.role === 'assistant' &&
        reply.failed &&
        !reply.rollback
      ) {
        ids.add(message.id);
      }
    });
    return ids;
  });

  constructor() {
    effect(() => this.history.save(this.messages()));
    this.messages()
      .filter((message) => this.isTracked(message))
      .forEach((message) => this.scheduleChangePoll(message.id, CHANGE_FIRST_POLL_DELAY_MS));
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
      void this.requestPlan(pendingId, text, payload.attachments.length, history);
      return;
    }
    void this.requestReply(pendingId, { text, attachments: payload.attachments }, history);
  }

  retry(id: string): void {
    const messages = this.messages();
    const index = messages.findIndex((message) => message.id === id);
    const request = messages[index];
    const reply = messages[index + 1];
    if (this.busy() || !this.retryableIds().has(id) || !request || !reply) {
      return;
    }

    const history = this.contextTurns(messages.slice(0, index)).slice(-MAX_HISTORY_TURNS);
    this.updateMessage(reply.id, { text: '', pending: true, failed: false });
    this.busy.set(true);

    if (request.edit) {
      void this.requestPlan(reply.id, request.text, request.attachments.length, history);
      return;
    }
    void this.requestReply(
      reply.id,
      { text: request.text, attachments: request.attachments },
      history,
    );
  }

  rollback(): void {
    if (this.busy()) {
      return;
    }
    const pendingId = this.appendPending(true);
    this.updateMessage(pendingId, { rollback: true });
    void this.requestRollback(pendingId);
  }

  toggleStep(messageId: string, stepId: string): void {
    const plan = this.findMessage(messageId)?.plan;
    if (!plan || this.findMessage(messageId)?.changeState !== 'plan') {
      return;
    }
    this.updateMessage(messageId, {
      plan: {
        ...plan,
        steps: plan.steps.map((step) =>
          step.id === stepId ? { ...step, selected: !step.selected } : step,
        ),
      },
    });
  }

  confirmChange(id: string): void {
    const message = this.findMessage(id);
    const plan = message?.plan;
    if (this.busy() || message?.changeState !== 'plan' || !plan) {
      return;
    }
    const request = this.composeRequest(plan);
    if (!request) {
      return;
    }
    const pendingId = this.appendPending(true);
    void this.requestChange(pendingId, request);
  }

  cancelChange(id: string): void {
    if (this.findMessage(id)?.changeState === 'plan') {
      this.updateMessage(id, { plan: undefined });
      this.applyChangeState(id, 'cancelled');
    }
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
    ]);
    return this.appendPending(edit);
  }

  private appendPending(edit: boolean): string {
    const pendingId = crypto.randomUUID();
    this.messages.update((messages) => [
      ...messages,
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
      this.updateMessage(pendingId, { text: reply, pending: false, failed: false });
      this.lastReply.set({ id: pendingId, text: reply });
    } catch (error) {
      this.failPending(pendingId, error);
    }
  }

  private async requestPlan(
    pendingId: string,
    request: string,
    attachmentsCount: number,
    history: ApiTurn[],
  ): Promise<void> {
    if (attachmentsCount > 0) {
      this.failPending(pendingId, new ChatApiError(API_MESSAGES.editNoFiles));
      return;
    }
    try {
      const reply = await this.api.planChange(request, history);
      if (reply.steps.length === 0) {
        this.updateMessage(pendingId, { text: reply.summary, pending: false, failed: false });
        this.lastReply.set({ id: pendingId, text: reply.summary });
        return;
      }
      this.updateMessage(pendingId, {
        text: reply.summary,
        pending: false,
        failed: false,
        changeState: 'plan',
        plan: {
          request,
          steps: reply.steps.map((text) => ({ id: crypto.randomUUID(), text, selected: true })),
        },
      });
      this.lastReply.set({ id: pendingId, text: reply.summary });
    } catch (error) {
      this.failPending(pendingId, error);
    }
  }

  private composeRequest(plan: ChangePlan): string | undefined {
    const approved = plan.steps.filter((step) => step.selected);
    if (approved.length === 0) {
      return undefined;
    }
    const list = approved.map((step, index) => `${index + 1}. ${step.text}`).join('\n');
    const tail = `\n\n${CHANGE_REQUEST_STEPS_HEADER}\n${list}`;
    const room = CHANGE_REQUEST_MAX_RUNES - Array.from(tail).length;
    const original = Array.from(plan.request);
    const head =
      original.length > room
        ? `${original.slice(0, Math.max(room - 1, 0)).join('')}…`
        : plan.request;
    return `${head}${tail}`;
  }

  private async requestChange(pendingId: string, request: string): Promise<void> {
    try {
      const reply = await this.api.requestChange(request);
      this.updateMessage(pendingId, {
        text: CHANGE_STATE_TEXT.queued,
        pending: false,
        failed: false,
        link: reply.url,
        linkLabel: CHANGE_LIST_LINK_LABEL,
        changeId: reply.id,
        changeState: 'queued',
      });
      this.scheduleChangePoll(pendingId, CHANGE_FIRST_POLL_DELAY_MS);
    } catch (error) {
      this.failPending(pendingId, error);
    }
  }

  private async requestRollback(pendingId: string): Promise<void> {
    try {
      const reply = await this.api.requestRollback();
      this.updateMessage(pendingId, {
        text: ROLLBACK_STATE_TEXT.queued,
        pending: false,
        failed: false,
        link: reply.url,
        linkLabel: CHANGE_LIST_LINK_LABEL,
        changeId: reply.id,
        changeState: 'queued',
      });
      this.scheduleChangePoll(pendingId, CHANGE_FIRST_POLL_DELAY_MS);
    } catch (error) {
      this.failPending(pendingId, error);
    }
  }

  private scheduleChangePoll(messageId: string, delayMs: number): void {
    setTimeout(() => void this.pollChange(messageId), delayMs);
  }

  private async pollChange(messageId: string): Promise<void> {
    const message = this.findMessage(messageId);
    if (!message?.changeId || !this.isTracked(message)) {
      return;
    }
    if (this.isStale(message)) {
      this.applyChangeState(messageId, 'unknown');
      return;
    }

    if (this.access.granted()) {
      try {
        const status = message.rollback
          ? await this.api.rollbackStatus(message.changeId)
          : await this.api.changeStatus(message.changeId);
        this.applyChangeStatus(messageId, status);
      } catch (error) {
        if (error instanceof ChatApiError && error.unauthorized) {
          this.access.clear();
        }
      }
    }

    const updated = this.findMessage(messageId);
    if (updated && this.isTracked(updated)) {
      this.scheduleChangePoll(messageId, this.pollInterval(updated.changeState));
    }
  }

  private applyChangeStatus(messageId: string, status: ChangeStatus): void {
    const hasPull = status.pullUrl !== undefined;
    const patch: Partial<ChatMessage> = {
      text: this.stateText(messageId, status.state),
      changeState: status.state,
    };
    if (hasPull) {
      patch.link = status.pullUrl;
      patch.linkLabel = `Открыть pull request №${status.pullNumber}`;
    } else if (status.state === 'failed' && status.runUrl) {
      patch.link = status.runUrl;
      patch.linkLabel = CHANGE_RUN_LINK_LABEL;
    }
    this.updateMessage(messageId, patch);
  }

  private applyChangeState(messageId: string, state: ChangeState): void {
    this.updateMessage(messageId, { text: this.stateText(messageId, state), changeState: state });
  }

  private stateText(messageId: string, state: ChangeState): string {
    if (this.findMessage(messageId)?.rollback && state !== 'plan' && state !== 'cancelled') {
      return ROLLBACK_STATE_TEXT[state];
    }
    return CHANGE_STATE_TEXT[state];
  }

  private isTracked(message: ChatMessage): boolean {
    return (
      message.changeId !== undefined &&
      message.changeState !== undefined &&
      CHANGE_TRACKED_STATES.includes(message.changeState)
    );
  }

  private isStale(message: ChatMessage): boolean {
    const active =
      message.changeState !== undefined && CHANGE_ACTIVE_STATES.includes(message.changeState);
    const limit = active ? CHANGE_ACTIVE_LIMIT_MS : CHANGE_REVIEW_LIMIT_MS;
    return Date.now() - message.createdAt.getTime() > limit;
  }

  private pollInterval(state: ChangeState | undefined): number {
    const active = state !== undefined && CHANGE_ACTIVE_STATES.includes(state);
    return active ? CHANGE_ACTIVE_POLL_INTERVAL_MS : CHANGE_REVIEW_POLL_INTERVAL_MS;
  }

  private failPending(id: string, error: unknown): void {
    if (error instanceof ChatApiError && error.unauthorized) {
      this.access.clear();
    }
    const message = error instanceof ChatApiError ? error.message : String(error);
    this.updateMessage(id, { text: message, pending: false, failed: true });
  }

  private findMessage(id: string): ChatMessage | undefined {
    return this.messages().find((message) => message.id === id);
  }

  private updateMessage(id: string, patch: Partial<ChatMessage>): void {
    this.messages.update((messages) =>
      messages.map((message) => (message.id === id ? { ...message, ...patch } : message)),
    );
    if (patch.pending === false) {
      this.busy.set(false);
    }
  }

  private restoreMessages(): ChatMessage[] {
    const restored = this.history.load();
    const ordered = [...restored].sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
    return ordered.length > 0 ? ordered : [this.createWelcomeMessage()];
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

  private contextTurns(messages: ChatMessage[]): ApiTurn[] {
    return messages
      .filter((message) => !message.pending && !message.failed && message.text.trim().length > 0)
      .map((message) => ({ role: message.role, text: this.editTurnText(message) }));
  }

  private buildHistory(): ApiTurn[] {
    return this.contextTurns(this.messages()).slice(-MAX_HISTORY_TURNS);
  }

  private editTurnText(message: ChatMessage): string {
    const text = message.text.trim();
    const steps = message.plan?.steps ?? [];
    if (steps.length === 0) {
      return text;
    }
    const list = steps.map((step, index) => `${index + 1}. ${step.text}`).join('\n');
    return `${text}\n\n${CHANGE_PLAN_STEPS_HEADER}\n${list}`;
  }
}
