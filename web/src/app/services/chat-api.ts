import { HttpClient, HttpErrorResponse, HttpHeaders } from '@angular/common/http';
import { Service, inject, isDevMode } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import {
  API_CHANGE_PATH,
  API_CHAT_PATH,
  API_DEV_URL,
  API_MESSAGES,
  API_PRODUCTION_URL,
  HTTP_OFFLINE,
  HTTP_UNAUTHORIZED,
} from '../constants';
import {
  ApiErrorBody,
  ApiReply,
  ApiTurn,
  ChangeReply,
  ChangeStatus,
  ChatApiError,
  ComposerPayload,
} from '../models';
import { Access } from './access';

@Service()
export class ChatApi {
  private readonly http = inject(HttpClient);
  private readonly access = inject(Access);

  async ask(payload: ComposerPayload, history: ApiTurn[]): Promise<string> {
    const body = new FormData();
    body.append('text', payload.text);
    body.append('history', JSON.stringify(history));
    for (const attachment of payload.attachments) {
      body.append('files', attachment.file, attachment.name);
    }

    const response = await this.send<ApiReply>('POST', API_CHAT_PATH, body);
    return response.reply;
  }

  requestChange(request: string): Promise<ChangeReply> {
    return this.send<ChangeReply>('POST', API_CHANGE_PATH, { request });
  }

  changeStatus(id: string): Promise<ChangeStatus> {
    return this.send<ChangeStatus>('GET', `${API_CHANGE_PATH}/${encodeURIComponent(id)}`);
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
