import { toSpeechText } from './speech-text';

describe('toSpeechText', () => {
  it('drops escaped and plain quotes', () => {
    expect(toSpeechText('Нажмите \\"Войти\\" и «Готово»')).toBe('Нажмите Войти и Готово');
  });

  it('drops backslashes in paths', () => {
    expect(toSpeechText('Файл server\\.env')).toBe('Файл server .env');
  });

  it('drops markdown marks', () => {
    expect(toSpeechText('## Итог\n**Важно**: `код` и _курсив_\n> цитата')).toBe(
      'Итог\nВажно : код и курсив\nцитата',
    );
  });

  it('reads links by their text and bare urls as a word', () => {
    expect(toSpeechText('См. [заявки](https://github.com/x) и https://example.com/a')).toBe(
      'См. заявки и ссылка',
    );
  });

  it('keeps plain text as is', () => {
    expect(toSpeechText('Всё работает, командир.')).toBe('Всё работает, командир.');
  });
});
