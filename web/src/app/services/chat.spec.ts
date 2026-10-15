import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import {
  API_CHANGE_PATH,
  API_CHAT_PATH,
  API_DEV_URL,
  API_MESSAGES,
  CHAT_HISTORY_STORAGE_KEY,
} from '../constants';
import { Access } from './access';
import { Chat } from './chat';

describe('Chat', () => {
  let chat: Chat;
  let access: Access;
  let http: HttpTestingController;

  const chatUrl = `${API_DEV_URL}${API_CHAT_PATH}`;

  beforeEach(() => {
    localStorage.clear();
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    chat = TestBed.inject(Chat);
    access = TestBed.inject(Access);
    http = TestBed.inject(HttpTestingController);
    access.save('secret-pass');
  });

  afterEach(() => {
    http.verify();
    localStorage.clear();
  });

  it('starts with a welcome message from the assistant', () => {
    expect(chat.messages().length).toBe(1);
    expect(chat.messages()[0].role).toBe('assistant');
  });

  it('sends the message with the password and resolves the pending reply', async () => {
    chat.send({ text: 'Привет', attachments: [] });

    expect(chat.messages().length).toBe(3);
    expect(chat.messages()[1].text).toBe('Привет');
    expect(chat.messages()[2].pending).toBe(true);
    expect(chat.busy()).toBe(true);

    const request = http.expectOne(chatUrl);
    expect(request.request.method).toBe('POST');
    expect(request.request.headers.get('Authorization')).toBe('Bearer secret-pass');
    const body = request.request.body as FormData;
    expect(body.get('text')).toBe('Привет');
    request.flush({ reply: 'Здравствуйте' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    expect(chat.messages()[2].pending).toBe(false);
    expect(chat.messages()[2].failed).toBe(false);
    expect(chat.messages()[2].text).toBe('Здравствуйте');
    expect(chat.busy()).toBe(false);
  });

  it('sends earlier finished turns as history', async () => {
    chat.send({ text: 'Первый', attachments: [] });
    http.expectOne(chatUrl).flush({ reply: 'Ответ' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    chat.send({ text: 'Второй', attachments: [] });
    const body = http.expectOne(chatUrl).request.body as FormData;
    const history = JSON.parse(body.get('history') as string) as { role: string; text: string }[];

    expect(history.map((turn) => turn.text)).toContain('Первый');
    expect(history.map((turn) => turn.text)).toContain('Ответ');
    expect(history.map((turn) => turn.text)).not.toContain('Второй');
  });

  it('marks the reply as failed and clears the password on 401', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    http
      .expectOne(chatUrl)
      .flush({ error: 'Неверный пароль' }, { status: 401, statusText: 'Unauthorized' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    expect(chat.messages()[2].failed).toBe(true);
    expect(chat.messages()[2].text).toBe(API_MESSAGES.unauthorized);
    expect(access.granted()).toBe(false);
  });

  it('shows the server error text on other failures', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    http
      .expectOne(chatUrl)
      .flush({ error: 'Файлы слишком большие' }, { status: 413, statusText: 'Too Large' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    expect(chat.messages()[2].failed).toBe(true);
    expect(chat.messages()[2].text).toBe('Файлы слишком большие');
    expect(access.granted()).toBe(true);
  });

  it('removes a finished message but keeps a pending one', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    const [, userMessage, pendingMessage] = chat.messages();

    chat.remove(pendingMessage.id);
    expect(chat.messages().length).toBe(3);

    http.expectOne(chatUrl).flush({ reply: 'Ок' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    chat.remove(userMessage.id);
    expect(chat.messages().map((message) => message.id)).not.toContain(userMessage.id);
    expect(chat.messages().length).toBe(2);
  });

  it('saves the conversation and restores it in a new session', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    http.expectOne(chatUrl).flush({ reply: 'Здравствуйте' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));
    TestBed.tick();

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    const restored = TestBed.inject(Chat);

    expect(restored.messages().map((message) => message.text)).toContain('Привет');
    expect(restored.messages().map((message) => message.text)).toContain('Здравствуйте');
  });

  it('does not persist failed replies', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    http.expectOne(chatUrl).flush({ error: 'Сбой' }, { status: 500, statusText: 'Error' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));
    TestBed.tick();

    const stored = JSON.parse(localStorage.getItem(CHAT_HISTORY_STORAGE_KEY) ?? '[]') as {
      text: string;
    }[];
    expect(stored.map((message) => message.text)).toContain('Привет');
    expect(stored.map((message) => message.text)).not.toContain('Сбой');
  });

  it('clears the conversation back to the welcome message', async () => {
    chat.send({ text: 'Привет', attachments: [] });
    http.expectOne(chatUrl).flush({ reply: 'Ок' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    chat.clear();
    TestBed.tick();

    expect(chat.messages().length).toBe(1);
    expect(chat.messages()[0].role).toBe('assistant');
    const stored = JSON.parse(localStorage.getItem(CHAT_HISTORY_STORAGE_KEY) ?? '[]') as unknown[];
    expect(stored.length).toBe(1);
  });

  it('does not clear while a reply is pending', () => {
    chat.send({ text: 'Привет', attachments: [] });

    chat.clear();

    expect(chat.messages().length).toBe(3);
    http.expectOne(chatUrl).flush({ reply: 'Ок' });
  });

  describe('edit mode', () => {
    const changeUrl = `${API_DEV_URL}${API_CHANGE_PATH}`;

    it('sends the message as a change request, shows the link and returns to ask mode', async () => {
      chat.toggleEditMode();
      expect(chat.mode()).toBe('edit');

      chat.send({ text: 'Сделай кнопку красной', attachments: [] });
      expect(chat.mode()).toBe('ask');

      const request = http.expectOne(changeUrl);
      expect(request.request.body).toEqual({ request: 'Сделай кнопку красной' });
      expect(request.request.headers.get('Authorization')).toBe('Bearer secret-pass');
      request.flush({ message: 'Заявка принята', url: 'https://github.com/me/app/pulls' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      const reply = chat.messages()[2];
      expect(reply.text).toBe('Заявка принята');
      expect(reply.link).toBe('https://github.com/me/app/pulls');
      expect(reply.edit).toBe(true);
      expect(reply.failed).toBe(false);
    });

    it('keeps change requests out of the conversation history', async () => {
      chat.toggleEditMode();
      chat.send({ text: 'Сделай кнопку красной', attachments: [] });
      http.expectOne(changeUrl).flush({ message: 'Заявка принята', url: 'u' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      chat.send({ text: 'Обычный вопрос', attachments: [] });
      const body = http.expectOne(chatUrl).request.body as FormData;
      const history = JSON.parse(body.get('history') as string) as { text: string }[];

      expect(history.map((turn) => turn.text)).not.toContain('Сделай кнопку красной');
      expect(history.map((turn) => turn.text)).not.toContain('Заявка принята');
    });

    it('refuses attachments in edit mode without calling the server', async () => {
      chat.toggleEditMode();
      const file = new File(['x'], 'a.png', { type: 'image/png' });

      chat.send({
        text: 'Поправь',
        attachments: [{ id: '1', name: 'a.png', mimeType: 'image/png', size: 1, file }],
      });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      http.expectNone(changeUrl);
      expect(chat.messages()[2].failed).toBe(true);
      expect(chat.messages()[2].text).toBe(API_MESSAGES.editNoFiles);
    });

    it('shows the server explanation when editing is not configured', async () => {
      chat.toggleEditMode();
      chat.send({ text: 'Поправь', attachments: [] });
      http
        .expectOne(changeUrl)
        .flush(
          { error: 'Правка приложения не настроена на сервере' },
          { status: 501, statusText: 'x' },
        );
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].failed).toBe(true);
      expect(chat.messages()[2].text).toBe('Правка приложения не настроена на сервере');
      expect(chat.mode()).toBe('ask');
    });

    it('does not switch the mode while a reply is pending', () => {
      chat.send({ text: 'Привет', attachments: [] });

      chat.toggleEditMode();

      expect(chat.mode()).toBe('ask');
      http.expectOne(chatUrl).flush({ reply: 'Ок' });
    });
  });

  it('ignores empty messages without attachments', () => {
    chat.send({ text: '   ', attachments: [] });

    expect(chat.messages().length).toBe(1);
    expect(chat.busy()).toBe(false);
  });
});
