import { HttpClient, HttpErrorResponse, HttpHeaders } from '@angular/common/http';
import { Service, inject, isDevMode } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import {
  API_CHAT_PATH,
  API_DEV_URL,
  API_MESSAGES,
  API_PRODUCTION_URL,
  HTTP_OFFLINE,
  HTTP_UNAUTHORIZED,
} from '../constants';
import { ApiErrorBody, ApiReply, ApiTurn, ChatApiError, ComposerPayload } from '../models';
import { Access } from './access';

@Service()
export class ChatApi {
  private readonly http = inject(HttpClient);
  private readonly access = inject(Access);

  async ask(payload: ComposerPayload, history: ApiTurn[]): Promise<string> {
    const baseUrl = isDevMode() ? API_DEV_URL : API_PRODUCTION_URL;
    if (!baseUrl) {
      throw new ChatApiError(API_MESSAGES.notConfigured);
    }

    const body = new FormData();
    body.append('text', payload.text);
    body.append('history', JSON.stringify(history));
    for (const attachment of payload.attachments) {
      body.append('files', attachment.file, attachment.name);
    }

    try {
      const response = await firstValueFrom(
        this.http.post<ApiReply>(`${baseUrl}${API_CHAT_PATH}`, body, {
          headers: new HttpHeaders({ Authorization: `Bearer ${this.access.password()}` }),
        }),
      );
      return response.reply;
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
