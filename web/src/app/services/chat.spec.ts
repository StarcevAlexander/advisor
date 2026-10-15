import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { API_CHAT_PATH, API_DEV_URL, API_MESSAGES } from '../constants';
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
    await Promise.resolve();
    await Promise.resolve();

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

  it('ignores empty messages without attachments', () => {
    chat.send({ text: '   ', attachments: [] });

    expect(chat.messages().length).toBe(1);
    expect(chat.busy()).toBe(false);
  });
});
