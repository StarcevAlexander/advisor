import { ChatMode, RecognitionState } from '../models';

export const RECOGNITION_LANG = 'ru-RU';

export const SPEECH_LANG = 'ru-RU';

export const MAX_ATTACHMENT_SIZE_BYTES = 20 * 1024 * 1024;

export const MAX_ATTACHMENTS = 8;

export const CHAT_HISTORY_STORAGE_KEY = 'advisor.chat-history';

export const MAX_STORED_MESSAGES = 100;

export const CLEAR_CONFIRM_TIMEOUT_MS = 3000;

export const IMAGE_MAX_SIDE_PX = 1600;

export const IMAGE_JPEG_QUALITY = 0.85;

export const IMAGE_KEEP_BELOW_BYTES = 400 * 1024;

export const SENDABLE_IMAGE_TYPES: readonly string[] = ['image/jpeg', 'image/png', 'image/webp'];

export const ACCEPTED_FILE_TYPES = 'image/*,application/pdf,text/plain,text/markdown,text/csv';

export const MIC_LABELS = {
  idle: 'Нажмите и говорите, я слушаю',
  listening: 'Слушаю… нажмите, чтобы отправить',
  unsupported: 'Голосовой ввод не поддерживается',
  denied: 'Нет доступа к микрофону',
  failed: 'Не расслышал, попробуйте ещё раз',
} as const satisfies Record<RecognitionState, string>;

export const COMPOSER_PLACEHOLDERS = {
  ask: 'Напишите сообщение…',
  edit: 'Опишите, что изменить в приложении…',
} as const satisfies Record<ChatMode, string>;

export const EDIT_MODE_HINT =
  'Режим правки: опишите идею, я распишу план по пунктам. Заявка уйдёт только после того, как вы выберете пункты и подтвердите.';

export const WELCOME_TEXT =
  'Я ваш советчик. Нажмите большую кнопку и скажите, что нужно, или напишите текстом и приложите файл.';
