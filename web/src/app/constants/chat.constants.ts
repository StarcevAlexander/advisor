export const SPEECH_LANG = 'ru-RU';

export const BYTES_IN_MB = 1024 * 1024;

export const MAX_ATTACHMENT_SIZE_BYTES = 100 * BYTES_IN_MB;

export const MAX_ATTACHMENTS_TOTAL_BYTES = 100 * BYTES_IN_MB;

export const MAX_ATTACHMENTS = 8;

export const ATTACHMENT_MESSAGES = {
  tooMany: `Можно приложить не больше ${MAX_ATTACHMENTS} файлов`,
  tooLarge: (name: string) =>
    `Файл «${name}» больше ${MAX_ATTACHMENT_SIZE_BYTES / BYTES_IN_MB} МБ, такой не отправить`,
  totalTooLarge: (name: string) =>
    `Файл «${name}» не добавлен: вместе вложения больше ${MAX_ATTACHMENTS_TOTAL_BYTES / BYTES_IN_MB} МБ`,
  unreadable: (name: string) =>
    `Не удалось прочитать файл «${name}». Если он в iCloud или на другом диске, сначала скачайте его на телефон и приложите снова`,
} as const;

export const CHAT_HISTORY_STORAGE_KEY = 'advisor.chat-history';

export const SPEAK_REPLIES_STORAGE_KEY = 'advisor.speak-replies';

export const DEVELOPER_MODE_STORAGE_KEY = 'advisor.developer-mode';

export const DEVELOPER_MODE_LABEL = 'Режим разработчика';

export const SPEAK_REPLIES_LABEL = 'Озвучивать ответы';

export const SETTINGS_TITLE = 'Настройки';

export const SETTINGS_OPEN_LABEL = 'Открыть настройки';

export const SETTINGS_CLOSE_LABEL = 'Закрыть настройки';

export const MAX_STORED_MESSAGES = 100;

export const MAX_HISTORY_TURNS = 20;

export const HISTORY_CONTEXT_LABEL = 'В беседе учитывается сообщений';

export const CLEAR_CONFIRM_TIMEOUT_MS = 3000;

export const IMAGE_MAX_SIDE_PX = 1600;

export const IMAGE_JPEG_QUALITY = 0.85;

export const IMAGE_KEEP_BELOW_BYTES = 400 * 1024;

export const SENDABLE_IMAGE_TYPES: readonly string[] = ['image/jpeg', 'image/png', 'image/webp'];

export const ACCEPTED_FILE_TYPES = 'image/*,application/pdf,text/plain,text/markdown,text/csv';

export const WELCOME_TEXT =
  'Я ваш советчик. Напишите, что нужно, и при необходимости приложите файл.';

export const VOICE_INPUT_LABELS = {
  start: 'Голосовой ввод',
  stop: 'Остановить запись',
} as const;
