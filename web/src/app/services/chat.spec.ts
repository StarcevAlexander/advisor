import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import {
  API_CHANGE_PATH,
  API_CHANGE_PLAN_PATH,
  API_CHAT_PATH,
  API_DEV_URL,
  API_MESSAGES,
  API_ROLLBACK_PATH,
  CHANGE_ACTIVE_LIMIT_MS,
  CHANGE_ACTIVE_POLL_INTERVAL_MS,
  CHANGE_FIRST_POLL_DELAY_MS,
  CHANGE_REVIEW_POLL_INTERVAL_MS,
  CHANGE_RUN_LINK_LABEL,
  CHANGE_STATE_TEXT,
  CHAT_HISTORY_STORAGE_KEY,
  ROLLBACK_STATE_TEXT,
} from '../constants';
import { Access } from './access';
import { Chat } from './chat';
import { Settings } from './settings';

describe('Chat', () => {
  let chat: Chat;
  let access: Access;
  let settings: Settings;
  let http: HttpTestingController;

  const chatUrl = `${API_DEV_URL}${API_CHAT_PATH}`;

  beforeEach(() => {
    localStorage.clear();
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    chat = TestBed.inject(Chat);
    access = TestBed.inject(Access);
    settings = TestBed.inject(Settings);
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

  it('keeps one conversation id between messages and starts a new one after clearing', async () => {
    chat.send({ text: 'Первый', attachments: [] });
    const first = http.expectOne(chatUrl);
    const firstId = (first.request.body as FormData).get('conversation') as string;
    first.flush({ reply: 'Ответ' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    chat.send({ text: 'Второй', attachments: [] });
    const second = http.expectOne(chatUrl);
    expect((second.request.body as FormData).get('conversation')).toBe(firstId);
    second.flush({ reply: 'Ещё ответ' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    chat.clear();
    chat.send({ text: 'Новый', attachments: [] });
    const third = http.expectOne(chatUrl);
    const thirdId = (third.request.body as FormData).get('conversation') as string;
    third.flush({ reply: 'Привет снова' });
    await vi.waitFor(() => expect(chat.busy()).toBe(false));

    expect(firstId).not.toBe('');
    expect(thirdId).not.toBe(firstId);
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

  describe('retry', () => {
    const unavailable = { error: 'Советчик временно недоступен, попробуйте ещё раз' };

    async function failFirstAsk(text = 'Привет'): Promise<string> {
      chat.send({ text, attachments: [] });
      http.expectOne(chatUrl).flush(unavailable, { status: 502, statusText: 'Bad Gateway' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
      return chat.messages()[1].id;
    }

    it('offers a retry only for the user message whose reply failed', async () => {
      const id = await failFirstAsk();

      expect(chat.retryableIds().has(id)).toBe(true);
      expect(chat.retryableIds().size).toBe(1);
    });

    it('resends the same message into the failed reply without adding messages', async () => {
      const id = await failFirstAsk();

      chat.retry(id);
      expect(chat.busy()).toBe(true);
      expect(chat.messages().length).toBe(3);
      expect(chat.messages()[2].pending).toBe(true);
      expect(chat.messages()[2].failed).toBe(false);
      expect(chat.retryableIds().size).toBe(0);

      const request = http.expectOne(chatUrl);
      expect((request.request.body as FormData).get('text')).toBe('Привет');
      request.flush({ reply: 'Ок' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
    });

    it('sends the earlier turns but not the retried exchange as history', async () => {
      chat.send({ text: 'Первый', attachments: [] });
      http.expectOne(chatUrl).flush({ reply: 'Ответ' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
      chat.send({ text: 'Второй', attachments: [] });
      http.expectOne(chatUrl).flush(unavailable, { status: 502, statusText: 'x' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      chat.retry(chat.messages()[3].id);
      const body = http.expectOne(chatUrl).request.body as FormData;
      const history = JSON.parse(body.get('history') as string) as { text: string }[];

      expect(history.map((turn) => turn.text)).toContain('Ответ');
      expect(history.map((turn) => turn.text)).not.toContain('Второй');
    });

    it('shows the answer in the same bubble when the retry succeeds', async () => {
      const id = await failFirstAsk();
      const replyId = chat.messages()[2].id;

      chat.retry(id);
      http.expectOne(chatUrl).flush({ reply: 'Здравствуйте' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages().length).toBe(3);
      expect(chat.messages()[2].id).toBe(replyId);
      expect(chat.messages()[2].text).toBe('Здравствуйте');
      expect(chat.messages()[2].failed).toBe(false);
      expect(chat.retryableIds().size).toBe(0);
    });

    it('keeps the retry available when it fails again', async () => {
      const id = await failFirstAsk();

      chat.retry(id);
      http.expectOne(chatUrl).flush(unavailable, { status: 502, statusText: 'x' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].failed).toBe(true);
      expect(chat.retryableIds().has(id)).toBe(true);
    });

    it('does nothing while another request is pending', async () => {
      const id = await failFirstAsk();
      chat.send({ text: 'Ещё', attachments: [] });

      chat.retry(id);

      expect(chat.messages().length).toBe(5);
      http.expectOne(chatUrl).flush({ reply: 'Ок' });
    });

    it('retries an edit request through the planner', async () => {
      settings.setDeveloperMode(true);
      chat.send({ text: 'Сделай кнопку красной', attachments: [] });
      http
        .expectOne(`${API_DEV_URL}${API_CHANGE_PLAN_PATH}`)
        .flush(unavailable, { status: 502, statusText: 'x' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      chat.retry(chat.messages()[1].id);
      const request = http.expectOne(`${API_DEV_URL}${API_CHANGE_PLAN_PATH}`);
      expect((request.request.body as { request: string }).request).toBe('Сделай кнопку красной');
      request.flush({ summary: 'Сделаю', steps: ['Кнопка станет красной'] });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].changeState).toBe('plan');
    });
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
    const planUrl = `${API_DEV_URL}${API_CHANGE_PLAN_PATH}`;
    const plan = {
      summary: 'Сделаю кнопку красной',
      steps: ['Кнопка станет красной', 'Текст станет белым'],
    };

    async function proposeIdea(text = 'Сделай кнопку красной'): Promise<string> {
      settings.setDeveloperMode(true);
      chat.send({ text, attachments: [] });
      http.expectOne(planUrl).flush(plan);
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
      return chat.messages()[2].id;
    }

    it('thinks the idea over first and shows the plan with every step selected', async () => {
      settings.setDeveloperMode(true);
      expect(chat.mode()).toBe('edit');

      chat.send({ text: 'Сделай кнопку красной', attachments: [] });
      expect(chat.mode()).toBe('edit');
      const request = http.expectOne(planUrl);
      const sent = request.request.body as { request: string; history: { text: string }[] };
      expect(sent.request).toBe('Сделай кнопку красной');
      expect(sent.history.map((turn) => turn.text)).not.toContain('Сделай кнопку красной');
      expect(request.request.headers.get('Authorization')).toBe('Bearer secret-pass');
      request.flush(plan);
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      http.expectNone(changeUrl);
      const reply = chat.messages()[2];
      expect(reply.text).toBe('Сделаю кнопку красной');
      expect(reply.changeState).toBe('plan');
      expect(reply.edit).toBe(true);
      expect(reply.plan?.steps.map((step) => step.text)).toEqual(plan.steps);
      expect(reply.plan?.steps.every((step) => step.selected)).toBe(true);
    });

    it('treats a plan without steps as a plain answer and offers nothing to send', async () => {
      settings.setDeveloperMode(true);
      chat.send({ text: 'Может, что-нибудь поменять?', attachments: [] });
      http.expectOne(planUrl).flush({ summary: 'Уточните, что именно поменять', steps: [] });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      const reply = chat.messages()[2];
      expect(reply.text).toBe('Уточните, что именно поменять');
      expect(reply.changeState).toBeUndefined();
      expect(reply.plan).toBeUndefined();
    });

    it('sends only the approved steps as the change request', async () => {
      const id = await proposeIdea();
      const [first] = chat.messages()[2].plan?.steps ?? [];

      chat.toggleStep(id, first.id);
      chat.confirmChange(id);

      const request = http.expectOne(changeUrl);
      const body = request.request.body as { request: string };
      expect(body.request).toContain('Сделай кнопку красной');
      expect(body.request).toContain('1. Текст станет белым');
      expect(body.request).not.toContain('Кнопка станет красной');
      request.flush({
        id: '0123456789ab',
        message: 'Заявка принята',
        url: 'https://github.com/me/app/pulls',
      });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      const reply = chat.messages()[3];
      expect(reply.text).toBe(CHANGE_STATE_TEXT.queued);
      expect(reply.changeId).toBe('0123456789ab');
      expect(reply.changeState).toBe('queued');
      expect(reply.link).toBe('https://github.com/me/app/pulls');
      expect(reply.plan).toBeUndefined();
      expect(reply.failed).toBe(false);
    });

    it('closes the plan once its request is sent so it cannot be sent twice', async () => {
      const id = await proposeIdea();

      chat.confirmChange(id);
      http
        .expectOne(changeUrl)
        .flush({ id: '0123456789ab', message: 'ok', url: 'https://github.com/me/app/pulls' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].changeState).toBeUndefined();
      expect(chat.messages()[2].plan?.steps.length).toBe(2);
      chat.confirmChange(id);
      http.expectNone(changeUrl);
    });

    it('keeps the plan open when sending the request fails', async () => {
      const id = await proposeIdea();

      chat.confirmChange(id);
      http.expectOne(changeUrl).flush({ error: 'boom' }, { status: 502, statusText: 'x' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].changeState).toBe('plan');
      expect(chat.messages()[3].failed).toBe(true);
      chat.confirmChange(id);
      http.expectOne(changeUrl).flush({ error: 'boom' }, { status: 502, statusText: 'x' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
    });

    it('does not send when no step is approved', async () => {
      const id = await proposeIdea();
      chat.messages()[2].plan?.steps.forEach((step) => chat.toggleStep(id, step.id));

      chat.confirmChange(id);

      http.expectNone(changeUrl);
      expect(chat.messages()[2].changeState).toBe('plan');
    });

    it('drops the plan on cancel without sending anything', async () => {
      const id = await proposeIdea();

      chat.cancelChange(id);
      chat.confirmChange(id);

      http.expectNone(changeUrl);
      expect(chat.messages()[2].changeState).toBe('cancelled');
      expect(chat.messages()[2].text).toBe(CHANGE_STATE_TEXT.cancelled);
      expect(chat.messages()[2].plan).toBeUndefined();
    });

    it('stays in developer mode across sessions and keeps its messages', async () => {
      await proposeIdea();
      TestBed.tick();

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideHttpClient(), provideHttpClientTesting()],
      });
      const restored = TestBed.inject(Chat);

      expect(restored.mode()).toBe('edit');
      expect(restored.messages().map((message) => message.text)).toContain('Сделай кнопку красной');
    });

    it('keeps the plan across sessions', async () => {
      await proposeIdea();
      TestBed.tick();

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideHttpClient(), provideHttpClientTesting()],
      });
      const restored = TestBed.inject(Chat);

      const reply = restored.messages()[2];
      expect(reply.changeState).toBe('plan');
      expect(reply.plan?.steps.length).toBe(2);
    });

    it('includes edit mode messages in the conversation history', async () => {
      const id = await proposeIdea();
      chat.confirmChange(id);
      http.expectOne(changeUrl).flush({ message: 'Заявка принята', url: 'u' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      settings.setDeveloperMode(false);
      chat.send({ text: 'Обычный вопрос', attachments: [] });
      const body = http.expectOne(chatUrl).request.body as FormData;
      const history = JSON.parse(body.get('history') as string) as { text: string }[];

      expect(history.map((turn) => turn.text)).toContain('Сделай кнопку красной');
      expect(history.some((turn) => turn.text.includes('Сделаю кнопку красной'))).toBe(true);
    });

    it('gives the planner the whole earlier dialogue with the proposed steps', async () => {
      chat.send({ text: 'Обычный вопрос', attachments: [] });
      http.expectOne(chatUrl).flush({ reply: 'Обычный ответ' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
      await proposeIdea();
      expect(chat.mode()).toBe('edit');

      chat.send({ text: 'Пусть будет тёмно-красной', attachments: [] });
      const body = http.expectOne(planUrl).request.body as {
        request: string;
        history: { role: string; text: string }[];
      };

      expect(body.request).toBe('Пусть будет тёмно-красной');
      const texts = body.history.map((turn) => turn.text);
      expect(texts).toContain('Обычный вопрос');
      expect(texts).toContain('Сделай кнопку красной');
      const planTurn = body.history.find((turn) => turn.text.includes('Сделаю кнопку красной'));
      expect(planTurn?.role).toBe('assistant');
      expect(planTurn?.text).toContain('1. Кнопка станет красной');
    });

    it('refuses attachments in edit mode without calling the server', async () => {
      settings.setDeveloperMode(true);
      const file = new File(['x'], 'a.png', { type: 'image/png' });

      chat.send({
        text: 'Поправь',
        attachments: [{ id: '1', name: 'a.png', mimeType: 'image/png', size: 1, file }],
      });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      http.expectNone(planUrl);
      http.expectNone(changeUrl);
      expect(chat.messages()[2].failed).toBe(true);
      expect(chat.messages()[2].text).toBe(API_MESSAGES.editNoFiles);
    });

    it('shows the server explanation when editing is not configured', async () => {
      settings.setDeveloperMode(true);
      chat.send({ text: 'Поправь', attachments: [] });
      http
        .expectOne(planUrl)
        .flush(
          { error: 'Правка приложения не настроена на сервере' },
          { status: 501, statusText: 'x' },
        );
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      expect(chat.messages()[2].failed).toBe(true);
      expect(chat.messages()[2].text).toBe('Правка приложения не настроена на сервере');
    });
  });

  describe('rollback', () => {
    const rollbackId = '0123456789ab';
    const rollbackUrl = `${API_DEV_URL}${API_ROLLBACK_PATH}`;
    const statusUrl = `${rollbackUrl}/${rollbackId}`;
    const changeStatusUrl = `${API_DEV_URL}${API_CHANGE_PATH}/${rollbackId}`;

    async function startRollback(): Promise<void> {
      chat.rollback();
      http
        .expectOne(rollbackUrl)
        .flush({ id: rollbackId, message: 'ok', url: 'https://x.test/pulls' });
      await vi.advanceTimersByTimeAsync(0);
    }

    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('queues the rollback as a tracked message', async () => {
      chat.rollback();
      expect(chat.busy()).toBe(true);
      expect(chat.messages()[1].pending).toBe(true);
      expect(chat.messages()[1].rollback).toBe(true);

      const request = http.expectOne(rollbackUrl);
      expect(request.request.method).toBe('POST');
      expect(request.request.headers.get('Authorization')).toBe('Bearer secret-pass');
      request.flush({ id: rollbackId, message: 'ok', url: 'https://x.test/pulls' });
      await vi.advanceTimersByTimeAsync(0);

      const message = chat.messages()[1];
      expect(chat.busy()).toBe(false);
      expect(message.pending).toBe(false);
      expect(message.changeId).toBe(rollbackId);
      expect(message.changeState).toBe('queued');
      expect(message.text).toBe(ROLLBACK_STATE_TEXT.queued);
    });

    it('follows the rollback through its own status endpoint with rollback texts', async () => {
      await startRollback();

      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);
      http.expectNone(changeStatusUrl);
      http.expectOne(statusUrl).flush({ state: 'working' });
      await vi.advanceTimersByTimeAsync(0);
      expect(chat.messages()[1].text).toBe(ROLLBACK_STATE_TEXT.working);

      await vi.advanceTimersByTimeAsync(CHANGE_ACTIVE_POLL_INTERVAL_MS);
      http.expectOne(statusUrl).flush({ state: 'merged', pullUrl: 'https://pr/9', pullNumber: 9 });
      await vi.advanceTimersByTimeAsync(0);

      expect(chat.messages()[1].changeState).toBe('merged');
      expect(chat.messages()[1].text).toBe(ROLLBACK_STATE_TEXT.merged);
      expect(chat.messages()[1].linkLabel).toBe('Открыть pull request №9');

      await vi.advanceTimersByTimeAsync(CHANGE_REVIEW_POLL_INTERVAL_MS * 3);
      http.expectNone(statusUrl);
    });

    it('says there is nothing to roll back when the run changed nothing', async () => {
      await startRollback();

      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);
      http.expectOne(statusUrl).flush({ state: 'nochange' });
      await vi.advanceTimersByTimeAsync(0);

      expect(chat.messages()[1].text).toBe(ROLLBACK_STATE_TEXT.nochange);
    });

    it('uses rollback wording when the tracking gives up', async () => {
      await startRollback();

      await vi.advanceTimersByTimeAsync(CHANGE_ACTIVE_LIMIT_MS + 1);
      http.expectOne(statusUrl).flush({ state: 'working' });
      await vi.advanceTimersByTimeAsync(0);
      await vi.advanceTimersByTimeAsync(CHANGE_ACTIVE_POLL_INTERVAL_MS);

      expect(chat.messages()[1].changeState).toBe('unknown');
      expect(chat.messages()[1].text).toBe(ROLLBACK_STATE_TEXT.unknown);
    });

    it('shows the server error and does not offer a retry of the previous message', async () => {
      vi.useRealTimers();
      chat.send({ text: 'Привет', attachments: [] });
      http.expectOne(`${API_DEV_URL}${API_CHAT_PATH}`).flush({ reply: 'Здравствуйте' });
      await vi.waitFor(() => expect(chat.busy()).toBe(false));
      chat.remove(chat.messages()[2].id);

      chat.rollback();
      http
        .expectOne(rollbackUrl)
        .flush(
          { error: 'Правка приложения не настроена на сервере' },
          { status: 501, statusText: 'x' },
        );
      await vi.waitFor(() => expect(chat.busy()).toBe(false));

      const failed = chat.messages()[chat.messages().length - 1];
      expect(failed.failed).toBe(true);
      expect(failed.text).toBe('Правка приложения не настроена на сервере');
      expect(chat.retryableIds().size).toBe(0);
    });

    it('does nothing while another request is pending', () => {
      chat.send({ text: 'Привет', attachments: [] });

      chat.rollback();

      http.expectNone(rollbackUrl);
      http.expectOne(`${API_DEV_URL}${API_CHAT_PATH}`).flush({ reply: 'Ок' });
    });

    it('keeps tracking a rollback after the page is reopened', async () => {
      await startRollback();
      TestBed.tick();

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideHttpClient(), provideHttpClientTesting()],
      });
      const restored = TestBed.inject(Chat);
      const restoredHttp = TestBed.inject(HttpTestingController);
      TestBed.inject(Access).save('secret-pass');

      expect(restored.messages()[1].rollback).toBe(true);
      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);
      restoredHttp.expectOne(statusUrl).flush({ state: 'merged' });
      await vi.advanceTimersByTimeAsync(0);
      expect(restored.messages()[1].text).toBe(ROLLBACK_STATE_TEXT.merged);
    });
  });

  describe('change tracking', () => {
    const changeId = '0123456789ab';
    const changeUrl = `${API_DEV_URL}${API_CHANGE_PATH}`;
    const statusUrl = `${changeUrl}/${changeId}`;

    async function submitChange(): Promise<void> {
      settings.setDeveloperMode(true);
      chat.send({ text: 'Сделай кнопку красной', attachments: [] });
      http
        .expectOne(`${API_DEV_URL}${API_CHANGE_PLAN_PATH}`)
        .flush({ summary: 'План', steps: ['Шаг'] });
      await vi.advanceTimersByTimeAsync(0);
      chat.confirmChange(chat.messages()[2].id);
      http.expectOne(changeUrl).flush({ id: changeId, message: 'ok', url: 'https://x.test/pulls' });
      await vi.advanceTimersByTimeAsync(0);
    }

    async function pollOnce(delayMs: number, body: object): Promise<void> {
      await vi.advanceTimersByTimeAsync(delayMs);
      http.expectOne(statusUrl).flush(body);
      await vi.advanceTimersByTimeAsync(0);
    }

    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('follows the request until it is merged and then stops asking', async () => {
      await submitChange();

      await pollOnce(CHANGE_FIRST_POLL_DELAY_MS, { state: 'working' });
      expect(chat.messages()[3].changeState).toBe('working');
      expect(chat.messages()[3].text).toBe(CHANGE_STATE_TEXT.working);

      await pollOnce(CHANGE_ACTIVE_POLL_INTERVAL_MS, {
        state: 'ready',
        pullUrl: 'https://pr/7',
        pullNumber: 7,
      });
      expect(chat.messages()[3].changeState).toBe('ready');
      expect(chat.messages()[3].link).toBe('https://pr/7');
      expect(chat.messages()[3].linkLabel).toBe('Открыть pull request №7');

      await pollOnce(CHANGE_REVIEW_POLL_INTERVAL_MS, {
        state: 'merged',
        pullUrl: 'https://pr/7',
        pullNumber: 7,
      });
      expect(chat.messages()[3].changeState).toBe('merged');
      expect(chat.messages()[3].text).toBe(CHANGE_STATE_TEXT.merged);

      await vi.advanceTimersByTimeAsync(CHANGE_REVIEW_POLL_INTERVAL_MS * 3);
      http.expectNone(statusUrl);
    });

    it('links to the run log when the run failed', async () => {
      await submitChange();

      await pollOnce(CHANGE_FIRST_POLL_DELAY_MS, { state: 'failed', runUrl: 'https://run/1' });

      expect(chat.messages()[3].changeState).toBe('failed');
      expect(chat.messages()[3].link).toBe('https://run/1');
      expect(chat.messages()[3].linkLabel).toBe(CHANGE_RUN_LINK_LABEL);
    });

    it('stops asking when the message is deleted', async () => {
      await submitChange();

      chat.remove(chat.messages()[3].id);
      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS * 2);

      http.expectNone(statusUrl);
    });

    it('keeps waiting when a status check fails', async () => {
      await submitChange();

      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);
      http.expectOne(statusUrl).flush({ error: 'x' }, { status: 502, statusText: 'Bad Gateway' });
      await vi.advanceTimersByTimeAsync(0);
      expect(chat.messages()[3].changeState).toBe('queued');

      await pollOnce(CHANGE_ACTIVE_POLL_INTERVAL_MS, { state: 'working' });
      expect(chat.messages()[3].changeState).toBe('working');
    });

    it('resumes tracking a stored request in a new session', async () => {
      await submitChange();
      await pollOnce(CHANGE_FIRST_POLL_DELAY_MS, { state: 'working' });
      TestBed.tick();

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideHttpClient(), provideHttpClientTesting()],
      });
      const restored = TestBed.inject(Chat);
      const restoredHttp = TestBed.inject(HttpTestingController);

      expect(restored.messages().some((message) => message.changeState === 'working')).toBe(true);
      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);
      restoredHttp
        .expectOne(statusUrl)
        .flush({ state: 'ready', pullUrl: 'https://pr/9', pullNumber: 9 });
      await vi.advanceTimersByTimeAsync(0);

      expect(restored.messages().some((message) => message.changeState === 'ready')).toBe(true);
    });

    it('gives up on a request that stayed active for too long', async () => {
      await submitChange();
      TestBed.tick();

      vi.setSystemTime(Date.now() + CHANGE_ACTIVE_LIMIT_MS + 1000);
      await vi.advanceTimersByTimeAsync(CHANGE_FIRST_POLL_DELAY_MS);

      http.expectNone(statusUrl);
      expect(chat.messages()[3].changeState).toBe('unknown');
      expect(chat.messages()[3].text).toBe(CHANGE_STATE_TEXT.unknown);
    });
  });

  it('ignores empty messages without attachments', () => {
    chat.send({ text: '   ', attachments: [] });

    expect(chat.messages().length).toBe(1);
    expect(chat.busy()).toBe(false);
  });
});
