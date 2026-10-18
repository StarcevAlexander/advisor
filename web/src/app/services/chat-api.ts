import { HttpClient, HttpErrorResponse, HttpHeaders } from '@angular/common/http';
import { Service, inject, isDevMode } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import {
  API_CHANGE_PATH,
  API_CLIENT_ERROR_PATH,
  API_CHANGE_PLAN_PATH,
  API_CHAT_PATH,
  API_DEV_URL,
  API_MESSAGES,
  API_PRODUCTION_URL,
  API_ROLLBACK_PATH,
  HTTP_OFFLINE,
  HTTP_UNAUTHORIZED,
} from '../constants';
import {
  ApiErrorBody,
  ApiReply,
  ApiTurn,
  ChangePlanReply,
  ChangeReply,
  ChangeStatus,
  ChatApiError,
  ClientErrorReport,
  ComposerPayload,
} from '../models';
import { Access } from './access';
import { Conversation } from './conversation';

@Service()
export class ChatApi {
  private readonly http = inject(HttpClient);
  private readonly access = inject(Access);
  private readonly conversation = inject(Conversation);

  async reportClientError(report: ClientErrorReport): Promise<void> {
    if (!this.access.granted()) {
      return;
    }
    try {
      await this.send<unknown>('POST', API_CLIENT_ERROR_PATH, report);
    } catch {
      return;
    }
  }

  async ask(payload: ComposerPayload, history: ApiTurn[]): Promise<string> {
    const body = new FormData();
    body.append('text', payload.text);
    body.append('history', JSON.stringify(history));
    body.append('conversation', this.conversation.id());
    for (const attachment of payload.attachments) {
      body.append('files', attachment.file, attachment.name);
    }

    const response = await this.send<ApiReply>('POST', API_CHAT_PATH, body);
    return response.reply;
  }

  planChange(request: string, history: ApiTurn[]): Promise<ChangePlanReply> {
    return this.send<ChangePlanReply>('POST', API_CHANGE_PLAN_PATH, { request, history });
  }

  requestChange(request: string): Promise<ChangeReply> {
    return this.send<ChangeReply>('POST', API_CHANGE_PATH, { request });
  }

  changeStatus(id: string): Promise<ChangeStatus> {
    return this.send<ChangeStatus>('GET', `${API_CHANGE_PATH}/${encodeURIComponent(id)}`);
  }

  requestRollback(): Promise<ChangeReply> {
    return this.send<ChangeReply>('POST', API_ROLLBACK_PATH);
  }

  rollbackStatus(id: string): Promise<ChangeStatus> {
    return this.send<ChangeStatus>('GET', `${API_ROLLBACK_PATH}/${encodeURIComponent(id)}`);
  }

  private async send<T>(
    method: 'GET' | 'POST',
    path: string,
    body?: FormData | object,
  ): Promise<T> {
    const baseUrl = isDevMode() ? API_DEV_URL : API_PRODUCTION_URL;
    if (!baseUrl) {
      throw new ChatApiError(API_MESSAGES.notConfigured);
    }

    try {
      return await firstValueFrom(
        this.http.request<T>(method, `${baseUrl}${path}`, {
          body,
          headers: new HttpHeaders({ Authorization: `Bearer ${this.access.password()}` }),
        }),
      );
    } catch (error) {
      throw this.toApiError(error);
    }
  }

  private toApiError(error: unknown): ChatApiError {
    if (!(error instanceof HttpErrorResponse)) {
      return new ChatApiError(API_MESSAGES.failed);
    }
    if (error.status === HTTP_UNAUTHORIZED) {
      return new ChatApiError(API_MESSAGES.unauthorized, true);
    }
    if (error.status === HTTP_OFFLINE) {
      return new ChatApiError(API_MESSAGES.offline);
    }
    const serverMessage = (error.error as ApiErrorBody | null)?.error;
    return new ChatApiError(serverMessage || API_MESSAGES.failed);
  }
}
