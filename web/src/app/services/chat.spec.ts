import { TestBed } from '@angular/core/testing';
import { STUB_REPLY_DELAY_MS } from '../constants';
import { Chat } from './chat';

describe('Chat', () => {
  let chat: Chat;

  beforeEach(() => {
    vi.useFakeTimers();
    TestBed.configureTestingModule({});
    chat = TestBed.inject(Chat);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with a welcome message from the assistant', () => {
    expect(chat.messages().length).toBe(1);
    expect(chat.messages()[0].role).toBe('assistant');
  });

  it('adds the user message and a pending reply, then resolves it', () => {
    chat.send({ text: 'Привет', attachments: [] });

    expect(chat.messages().length).toBe(3);
    expect(chat.messages()[1].text).toBe('Привет');
    expect(chat.messages()[2].pending).toBe(true);
    expect(chat.busy()).toBe(true);

    vi.advanceTimersByTime(STUB_REPLY_DELAY_MS);

    expect(chat.messages()[2].pending).toBe(false);
    expect(chat.messages()[2].text).toContain('Привет');
    expect(chat.busy()).toBe(false);
  });

  it('removes a finished message but keeps a pending one', () => {
    chat.send({ text: 'Привет', attachments: [] });
    const [, userMessage, pendingMessage] = chat.messages();

    chat.remove(pendingMessage.id);
    expect(chat.messages().length).toBe(3);

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
